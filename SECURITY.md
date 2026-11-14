# Security policy

Cedarflake Ame is a local-first Windows image workbench. Protecting original media, private library
information and durable user decisions is part of its security boundary. The
[threat model](docs/architecture/threat-model.md) describes the implementation, trust boundaries
and remaining risks.

## Supported versions

Ame currently publishes prereleases for Windows 11 x64. Security fixes target `main` and the next
official release. Use the latest [published release](https://github.com/Cedarflake/Cedarflake-Ame/releases);
older prereleases do not have separately maintained security branches. Development builds and
forks must identify their exact source commit when reporting a problem.

Reports affecting older versions are welcome. Include the affected version and whether the problem
also occurs in the latest release; do not risk an irreplaceable library merely to reproduce it.

## Report a vulnerability

Use this repository's
[private vulnerability reporting form](https://github.com/Cedarflake/Cedarflake-Ame/security/advisories/new).
Keep exploit details, malicious samples and private library data out of public issues and pull
requests. Include the following in the private report:

- the release tag or commit, Windows version, and MSI, portable or development deployment;
- affected feature, expected behavior, observed impact and required attacker access;
- minimal reproduction steps using generated or disposable files;
- whether the journal service, cloud-backed roots or GPU analysis are involved;
- relevant redacted errors and, when useful, a patch or regression-test suggestion.

Do not attach a whole catalog, organization archive, crash dump, original collection, access token
or signed download URL. These can expose paths, images, locations, account details or credentials.
Agree on a transfer method before supplying a sensitive or potentially malicious sample.

Maintainers use the private report to assess impact, coordinate a fix or mitigation, and agree on
disclosure and reporter credit. Keep technical details private during that coordination. Ordinary
feature requests and non-sensitive defects can use public issues.

## What to report

Security-relevant failures include:

- code execution, process escape or excessive resource consumption triggered by media, metadata,
  models, archives or IPC input;
- unauthorized file changes, overwrites, deletion, source-root escape or cloud-placeholder hydration;
- journal-broker impersonation, cross-user disclosure or privilege escalation;
- replacement of authorized file-operation targets, replayed mutation authority, or loss of durable
  decisions through recovery or migration;
- leakage of private library data, or tampering with downloaded models, native runtimes, installers
  or release artifacts.

Use only systems and disposable data you control for reproduction. Do not exercise destructive
operations against someone else's library or test the release pipeline with production credentials.

## Distribution and local data

Current official releases are unsigned and have no automatic updater. Obtain packages from this
repository's releases and consult their release notes. Artifact hashes verify bytes against the
trusted release record; they do not provide an independent publisher signature. See the
[distribution decision](docs/architecture/0015-windows-release-distribution.md).

The MSI installs a constrained Windows journal service with administrator approval; the portable
ZIP installs no service. The desktop application and its file operations use the launching user's
rights. Run the desktop without elevation when browsing or processing media.
Catalogs, thumbnails, analysis data and exported organization archives can contain private
information. Ame does not provide application-level encryption for these stores; protect them with
Windows account permissions and storage protection appropriate to the machine.
