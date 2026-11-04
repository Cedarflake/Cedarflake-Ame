use super::*;

pub(super) struct MatchWork {
    pub evidence_id: i64,
    pub phase: MatchPhase,
    pub reference_revision: i64,
    pub query_variant: usize,
    pub band: usize,
    pub neighbor: usize,
    pub after_evidence_id: i64,
    pub after_group_id: String,
    pub after_variant: i64,
    pub found_reference: bool,
}

impl MatchWork {
    pub(super) fn restart(&mut self, phase: MatchPhase) {
        self.phase = phase;
        self.query_variant = 0;
        self.band = 0;
        self.reset_posting();
    }

    pub(super) fn next_band(&mut self, variants: usize) -> bool {
        self.reset_posting();
        if self.band < 3 {
            self.band += 1;
            return true;
        }
        self.band = 0;
        if self.query_variant + 1 < variants {
            self.query_variant += 1;
            return true;
        }
        false
    }

    fn reset_posting(&mut self) {
        self.neighbor = 0;
        self.after_evidence_id = 0;
        self.after_group_id.clear();
        self.after_variant = -1;
    }
}

pub(super) fn load(transaction: &Transaction<'_>) -> Result<Option<MatchWork>, ScanError> {
    let row = transaction.query_row(
        "SELECT evidence_id, phase, reference_revision, query_variant, band, neighbor,
                after_evidence_id, after_group_id, after_variant, found_reference
         FROM similarity_match_work WHERE evidence_id = (
           SELECT MIN(id) FROM (
             SELECT MIN(evidence_id) AS id FROM similarity_match_work WHERE phase = 'references'
             UNION ALL SELECT MIN(evidence_id) FROM similarity_match_work WHERE phase = 'peer'
             UNION ALL SELECT MIN(evidence_id) FROM similarity_match_work WHERE phase = 'members'))",
        [],
        |row| {
            Ok((row.get::<_, String>(1)?, MatchWork {
                evidence_id: row.get(0)?,
                phase: MatchPhase::Complete,
                reference_revision: row.get(2)?,
                query_variant: usize::from(row.get::<_, u8>(3)?),
                band: usize::from(row.get::<_, u8>(4)?),
                neighbor: usize::from(row.get::<_, u8>(5)?),
                after_evidence_id: row.get(6)?,
                after_group_id: row.get(7)?,
                after_variant: row.get(8)?,
                found_reference: row.get(9)?,
            }))
        },
    ).optional().map_err(database_error)?;
    row.map(|(phase, mut work)| {
        work.phase = match phase.as_str() {
            "references" => MatchPhase::References,
            "peer" => MatchPhase::Peer,
            "members" => MatchPhase::Members,
            _ => return Err(invalid()),
        };
        if work.band > 3 || work.query_variant > 7 || work.neighbor > 136 {
            return Err(invalid());
        }
        Ok(work)
    })
    .transpose()
}

pub(super) fn save(transaction: &Transaction<'_>, work: &MatchWork) -> Result<(), ScanError> {
    let phase = match work.phase {
        MatchPhase::References => "references",
        MatchPhase::Peer => "peer",
        MatchPhase::Members => "members",
        MatchPhase::Complete => "complete",
    };
    transaction.execute(
        "UPDATE similarity_match_work SET phase = ?2, reference_revision = ?3, query_variant = ?4,
           band = ?5, neighbor = ?6, after_evidence_id = ?7, after_group_id = ?8, after_variant = ?9,
           found_reference = ?10 WHERE evidence_id = ?1",
        params![
            work.evidence_id,
            phase,
            work.reference_revision,
            work.query_variant as i64,
            work.band as i64,
            work.neighbor as i64,
            work.after_evidence_id,
            work.after_group_id,
            work.after_variant,
            work.found_reference,
        ],
    ).map_err(database_error)?;
    Ok(())
}
