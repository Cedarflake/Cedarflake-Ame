use std::mem::{offset_of, size_of};
use std::ptr;

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
    CoUninitialize,
};
use windows::Win32::System::Search::{
    DBACCESSOR_ROWDATA, DBBINDING, DBPROP, DBPROP_COMMANDTIMEOUT, DBPROPSET, DBPROPSTATUS_OK,
    DBTYPE_FILETIME, DBTYPE_UI8, DBTYPE_WSTR, HACCESSOR, IAccessor, ICommandProperties,
    ICommandText, IDBCreateCommand, IDBCreateSession, IDBInitialize, IDataInitialize, IRowset,
    MSDAINITIALIZE,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Shell::PathCreateFromUrlW;
use windows::core::{Error, GUID, Interface, PCWSTR, PWSTR, Result, w};

const DBGUID_DEFAULT: GUID = GUID::from_u128(0xc8b521fb_5cf3_11ce_ade5_00aa0044773d);
const DBPROPSET_ROWSET: GUID = GUID::from_u128(0xc8b522be_5cf3_11ce_ade5_00aa0044773d);

pub(super) struct Apartment;

impl Apartment {
    pub(super) fn new() -> Result<Self> {
        // This adapter is called on a dedicated worker. All interfaces die before this guard.
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        }
        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

#[repr(C)]
struct RowBuffer {
    status: [u32; 3],
    path_bytes: usize,
    size: u64,
    modified_ticks: u64,
    path: [u16; 32_768],
}

pub(super) struct Row {
    pub path: String,
    pub size: Option<u64>,
    pub modified_ticks: Option<u64>,
}

pub(super) struct Cursor {
    rowset: IRowset,
    accessor: IAccessor,
    handle: HACCESSOR,
    rows: *mut usize,
    row_count: usize,
    next_row: usize,
    buffer: Box<RowBuffer>,
    decoded_path: Vec<u16>,
    _command: ICommandText,
    _session: IDBCreateCommand,
    _database: IDBInitialize,
}

impl Cursor {
    pub(super) fn open(query: &str) -> Result<Self> {
        // COM owns its interfaces. The buffers and bindings are stable until their calls return.
        unsafe {
            let initializer: IDataInitialize =
                CoCreateInstance(&MSDAINITIALIZE, None, CLSCTX_INPROC_SERVER)?;
            let mut source = None;
            initializer.GetDataSource(
                None,
                CLSCTX_INPROC_SERVER.0,
                w!("Provider=Search.CollatorDSO;Extended Properties='Application=Windows';"),
                &IDBInitialize::IID,
                &mut source,
            )?;
            let database: IDBInitialize = source.ok_or_else(Error::from_thread)?.cast()?;
            database.Initialize()?;
            let sessions: IDBCreateSession = database.cast()?;
            let session: IDBCreateCommand = sessions
                .CreateSession(None, &IDBCreateCommand::IID)?
                .cast()?;
            let command: ICommandText = session.CreateCommand(None, &ICommandText::IID)?.cast()?;
            let properties: ICommandProperties = command.cast()?;
            let mut timeout = DBPROP {
                dwPropertyID: DBPROP_COMMANDTIMEOUT.0 as u32,
                vValue: VARIANT::from(5_i32),
                ..DBPROP::default()
            };
            properties.SetProperties(&[DBPROPSET {
                rgProperties: &mut timeout,
                cProperties: 1,
                guidPropertySet: DBPROPSET_ROWSET,
            }])?;
            if timeout.dwStatus != DBPROPSTATUS_OK.0 as u32 {
                return Err(Error::from_hresult(windows::Win32::Foundation::E_FAIL));
            }
            let text: Vec<u16> = query.encode_utf16().chain(Some(0)).collect();
            command.SetCommandText(&DBGUID_DEFAULT, PCWSTR(text.as_ptr()))?;
            let mut result = None;
            command.Execute(None, &IRowset::IID, None, None, Some(&mut result))?;
            let rowset: IRowset = result.ok_or_else(Error::from_thread)?.cast()?;
            let accessor: IAccessor = rowset.cast()?;
            let bindings = [
                binding(
                    1,
                    offset_of!(RowBuffer, path),
                    65_536,
                    DBTYPE_WSTR.0 as u16,
                    0,
                    Some(offset_of!(RowBuffer, path_bytes)),
                ),
                binding(
                    2,
                    offset_of!(RowBuffer, size),
                    8,
                    DBTYPE_UI8.0 as u16,
                    4,
                    None,
                ),
                binding(
                    3,
                    offset_of!(RowBuffer, modified_ticks),
                    8,
                    DBTYPE_FILETIME.0 as u16,
                    8,
                    None,
                ),
            ];
            let mut handle = HACCESSOR::default();
            accessor.CreateAccessor(
                DBACCESSOR_ROWDATA.0 as u32,
                bindings.len(),
                bindings.as_ptr(),
                size_of::<RowBuffer>(),
                &mut handle,
                None,
            )?;
            Ok(Self {
                rowset,
                accessor,
                handle,
                rows: ptr::null_mut(),
                row_count: 0,
                next_row: 0,
                buffer: Box::new(RowBuffer {
                    status: [0; 3],
                    path_bytes: 0,
                    size: 0,
                    modified_ticks: 0,
                    path: [0; 32_768],
                }),
                decoded_path: vec![0; 32_768],
                _command: command,
                _session: session,
                _database: database,
            })
        }
    }

    pub(super) fn next(&mut self) -> Result<Option<Row>> {
        unsafe {
            if self.next_row == self.row_count {
                self.release_rows();
                // The generated slice wrapper models the returned allocation incorrectly; use
                // the SDK signature with an explicit row count and one owned output pointer.
                (self.rowset.vtable().GetNextRows)(
                    self.rowset.as_raw(),
                    0,
                    0,
                    256,
                    &mut self.row_count,
                    &mut self.rows,
                )
                .ok()?;
                if self.row_count == 0 {
                    return Ok(None);
                }
                if self.rows.is_null() || self.row_count > 256 {
                    return Err(Error::from_hresult(windows::core::HRESULT(
                        0x80004005_u32 as i32,
                    )));
                }
            }
            let row = *self.rows.add(self.next_row);
            self.next_row += 1;
            self.rowset
                .GetData(row, self.handle, ptr::from_mut(self.buffer.as_mut()).cast())?;
            if self.buffer.status[0] != 0
                || self.buffer.path_bytes >= 65_536
                || !self.buffer.path_bytes.is_multiple_of(2)
            {
                return Err(Error::from_hresult(windows::core::HRESULT(
                    0x80004005_u32 as i32,
                )));
            }
            let mut path_length = self.decoded_path.len() as u32;
            self.buffer.path[self.buffer.path_bytes / 2] = 0;
            PathCreateFromUrlW(
                PCWSTR(self.buffer.path.as_ptr()),
                PWSTR(self.decoded_path.as_mut_ptr()),
                &mut path_length,
                0,
            )?;
            let path_length = self
                .decoded_path
                .iter()
                .position(|value| *value == 0)
                .unwrap_or(self.decoded_path.len());
            let path = String::from_utf16(&self.decoded_path[..path_length])
                .map_err(|_| Error::from_hresult(windows::core::HRESULT(0x80004005_u32 as i32)))?;
            Ok(Some(Row {
                path,
                size: (self.buffer.status[1] == 0).then_some(self.buffer.size),
                modified_ticks: (self.buffer.status[2] == 0).then_some(self.buffer.modified_ticks),
            }))
        }
    }

    fn release_rows(&mut self) {
        if !self.rows.is_null() {
            unsafe {
                let _ = self.rowset.ReleaseRows(
                    self.row_count,
                    self.rows,
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                CoTaskMemFree(Some(self.rows.cast()));
            }
        }
        self.rows = ptr::null_mut();
        self.row_count = 0;
        self.next_row = 0;
    }
}

impl Drop for Cursor {
    fn drop(&mut self) {
        self.release_rows();
        unsafe {
            let _ = self.accessor.ReleaseAccessor(self.handle, None);
        }
    }
}

fn binding(
    ordinal: usize,
    offset: usize,
    bytes: usize,
    kind: u16,
    status: usize,
    length: Option<usize>,
) -> DBBINDING {
    DBBINDING {
        iOrdinal: ordinal,
        obValue: offset,
        obStatus: status,
        obLength: length.unwrap_or(0),
        dwPart: 1 | 4 | if length.is_some() { 2 } else { 0 },
        cbMaxLen: bytes,
        wType: kind,
        ..DBBINDING::default()
    }
}
