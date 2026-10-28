# PoiskContact Pro: Business Contact Discovery for Windows

[![Windows checks](https://github.com/madnessbrainsbl/poisk-contact/actions/workflows/ci.yml/badge.svg)](https://github.com/madnessbrainsbl/poisk-contact/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/madnessbrainsbl/poisk-contact)](https://github.com/madnessbrainsbl/poisk-contact/releases/latest)

PoiskContact Pro is a Windows desktop application for finding public business
contacts, reviewing website evidence, and managing a local contact database.
Search companies by location and industry, extract published email addresses and
phone numbers, and export reviewed records to CSV, vCard, or CRM formats.

Built with Rust and egui, PoiskContact combines a native desktop interface with
command-line tools. Email outreach is optional and uses your own mail settings.

[Download for Windows](https://github.com/madnessbrainsbl/poisk-contact/releases/latest)
| [Getting started](#getting-started)
| [Build from source](#build-from-source)
| [FAQ](#frequently-asked-questions)

## What can PoiskContact do?

| Task | Capabilities |
| --- | --- |
| Find businesses | Search by city, country, and industry with online autocomplete |
| Extract public contacts | Crawl company websites for published email, phone, and social links |
| Review evidence | Inspect source pages, merge duplicate records, and keep notes |
| Export contacts | JSON, CSV, vCard, AmoCRM, Bitrix24, Instantly, and Lemlist |
| Import research | JSON/CSV contacts, 2GIS CSV exports, and Chrome crawler evidence |
| Send email | Template previews, SMTP relay, follow-ups, and optional Direct MX |
| Automate collection | Command-line search and resumable business checkpoints |

Optional sources include public job boards, Telegram channels, and Google Maps
search results. Source coverage and availability vary; contact collection does
not establish mailbox ownership or permission to contact someone.

## Getting started

### Download and run on Windows

1. Download `PoiskContact-0.1.0-windows-x64.zip` from
   [GitHub Releases](https://github.com/madnessbrainsbl/poisk-contact/releases/latest).
2. Extract the complete archive into a new folder you can write to.
3. Run `start.bat` or `poisk_contact.exe` from that folder.
4. Open **Find contacts**, enter a location and industry, and start collection.
5. Review records and their sources in **Contacts**, then export the results.

The portable package includes the desktop app and command-line executables.
Rust and Visual Studio are only needed when building from source. Optional
browser integrations have separate installation requirements. The executables
are not digitally signed.

Use Windows 10 or Windows 11 x64 with an OpenGL-capable graphics driver and the
[Microsoft Visual C++ Redistributable for x64](https://aka.ms/vc14/vc_redist.x64.exe).
If Windows reports that `VCRUNTIME140.dll` is missing, install that runtime first.

### Configure optional email outreach

Enter your own From address, Reply-To address, and SMTP account in **Mail and
sending**. Edit the message in **Email template**, inspect its preview, and use a
single-recipient test before selecting recipients for a campaign.

New installations have an empty sender identity. Optional profile presets can
be configured by copying `profiles.example.json` to `profiles.json` and filling
in your own details. Existing saved settings and templates remain local.

## Command-line contact collection

The Windows release includes `cli.exe`, `collect_business.exe`, and
`applypilot_jobs.exe`. Run collection in a separate working folder when you want
to keep an existing database untouched.

```powershell
.\cli.exe --city "Berlin" --stack "Photography" --max 30
.\collect_business.exe --city "Krasnoyarsk" --max 100 --output-dir output/businesses
python -X utf8 tools/verify_business_collection.py output/businesses
```

The collector saves progress and resumes from the selected output directory.
The optional Python verifier uses the standard library with Python 3.10+ and
re-fetches source pages. Its location and phone checks currently target
Krasnoyarsk and Russian phone formats; review flagged results before using them.
The search CLI writes `contacts.json` and `contacts.csv` in its working directory
by default.

### Optional search backends

Public search providers are available without configuring an API key. You can
also set `SEARXNG_URL` for your own [SearXNG](https://docs.searxng.org/) instance
or `BRAVE_SEARCH_API_KEY` for the [Brave Search API](https://brave.com/search/api/).
Provider rate limits, coverage, and markup can affect results. An empty result
is not proof that no matching businesses exist.

## Optional browser integrations

### Render JavaScript pages with Scrapling

Run this once with Python 3.10+ to install the optional browser environment:

```powershell
powershell -ExecutionPolicy Bypass -File tools/setup_scrapling.ps1
```

The crawler can then render JavaScript-only company pages in an isolated
Chromium process. Normal contact and location checks still apply. Rendering is
limited to two workers, one page per company, and a 20-second page budget.
No personal browser profile is used. Set `POISK_SCRAPLING_PYTHON` to an empty
value to disable this optional renderer.

### Import 2GIS exports

Install [Parser2GIS](https://github.com/Eroloft/parser-2gis-new) separately, run
`start_2gis_web.bat`, export CSV, and import it in the desktop application.
The companion requires Python and Google Chrome.

### Import Chrome crawler evidence

Install [Chrome crawler](https://github.com/ir00nman/chrome-crawler) separately.
In **Find contacts**, expand **Chrome crawler (browser assistant)**, copy the
collection task, run it in the extension, and import its
`poisk-contact.chrome-crawler.v1` JSON response.

This integration uses manual task copy and JSON import. It does not
control the browser extension automatically. The entire returned batch is
validated before import, and imported contacts still require review. Configure
the extension's model separately. Companion licenses are listed in
[third-party notices](THIRD_PARTY_NOTICES.md).

## Build from source

The supported build target is Windows x64. Install
[Rust with rustup](https://www.rust-lang.org/tools/install) and the Visual Studio
Build Tools with **Desktop development with C++** and the Windows SDK.
The project uses stable Rust and includes `Cargo.lock`.

```powershell
git clone https://github.com/madnessbrainsbl/poisk-contact.git
cd poisk-contact
cargo build --locked --release --bins
.\start.bat
```

The first build downloads dependencies. `start.bat` also builds the desktop app
if no executable is available. The source equivalents of the CLI commands are
`cargo run --locked --bin cli -- ...` and
`cargo run --locked --release --bin collect_business -- ...`.

## Frequently asked questions

### Does PoiskContact upload my contact database?

PoiskContact stores its contact database, mail settings, profiles, suppression
lists, and exports in local files. Search and crawling send requests to external
providers and company websites. Sending email connects to your configured mail
provider or the recipient's mail server. Third-party browser companions have
their own data handling and model settings.

### Can I use PoiskContact without an email account?

Yes. Business discovery, contact review, imports, and exports can be used without
configuring a sender. Email delivery is a separate workflow and requires your
own sender identity and mail configuration.

### Does an MX record mean an address is verified?

No. An MX record indicates that a domain has mail infrastructure. A successful
SMTP address probe or accepted message does not establish mailbox ownership,
inbox placement, or delivery. Review the original source and any flagged records.

### What does Direct MX require?

Direct MX connects from your computer to recipient mail servers on outbound
TCP port 25. It needs an appropriate sending domain and network configuration.
STARTTLS and certificate verification are required. Optional DKIM signing uses
an installed OpenSSL executable; Git for Windows is detected automatically.
SMTP providers enforce their own sender and alias rules.

### Where should I report a problem?

Open a [GitHub issue](https://github.com/madnessbrainsbl/poisk-contact/issues) with
the application version, Windows version, and steps to reproduce the problem.
Redact addresses, credentials, contact records, and private paths from logs or
screenshots. Do not attach a working contact database or saved mail settings.

## Development checks

```powershell
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo test --locked --doc
python tools/test_scrapling_fetch.py ScraplingBridgeTest.test_detects_javascript_shell
```

CI runs formatting and tests on Windows. Live-provider and real-browser tests
marked `ignore` require an explicit run and are excluded from the default suite.
Examples such as `send_test_email` and `run_campaign` can send real email;
normal test commands do not execute their entry points.

## Local data and license

Back up local data before moving the application. Mail credentials are stored in
local settings files; protect those files with operating-system access controls.
Keep DKIM private keys outside the repository. Use collected contacts only where
you have permission and review the evidence before outreach.

The original PoiskContact source is published without an open-source license
grant. Dependencies and optional companions retain their respective licenses.
Windows release packages include dependency notices and license texts. See
[third-party notices](THIRD_PARTY_NOTICES.md) for companion licensing information.
