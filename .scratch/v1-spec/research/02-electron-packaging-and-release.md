# Electron packaging and cross-platform release, unsigned, from GitHub Actions

Research ticket: `.scratch/v1-spec/issues/02-electron-packaging-and-release.md`
Researched 2026-08-04 against live repositories, live registries and current vendor documentation,
not from memory.

Primary sources used: the electron-builder source tree (cloned at `53f649b5`, 2026-08-04) and its
shipped `website/docs`; the Electron Forge source tree (cloned at `36512372`, 2026-08-02) and its
published docs sitemap; the `electron/electron` docs in-repo; the GitHub REST API for measured
repository activity; the npm registry API; `actions/runner-images` README; the `github/docs`
content repository (raw Markdown, so the strings are the ones GitHub publishes); Apple's Platform
Security guide, Apple's macOS User Guide, Apple's support article 102445 and Apple's developer
news; Microsoft Learn and Microsoft Support; the Fedora Project wiki; `docs.appimage.org`; the
Node.js v24 API documentation. Where a claim rests on a secondary source or on inference, it says
so in place.

**Headline:** pick **electron-builder**, and the single decision that matters most is not the
build tool at all — it is that **a genuinely unsigned macOS build is a brick on Apple Silicon**.
Apple does not permit unsigned arm64 code to execute at all, electron-builder does *not* ad-hoc
sign automatically when it finds no certificate, and `macos-latest` is now an arm64 runner. The
result is the "damaged and can't be opened" dialog, which has **no Open Anyway path** — unlike the
"Apple cannot check…" dialog, which does. The fix is one config line plus two entitlements
(`mac.identity: "-"`), which converts an unrecoverable failure into a documented five-click
workaround. Two other findings move other tickets: **Fedora 45 (this autumn) will make `rpm`
refuse unsigned packages**, and **`node:sqlite` is built into Electron**, so SQLite need not cost a
native module at all.

---

## 1. electron-builder versus Electron Forge

### Measured activity, 2026-08-04

All figures below were pulled from the GitHub REST API and the npm registry API on 2026-08-04,
not from reputation.

| | electron-builder | Electron Forge |
|---|---|---|
| Repo | `electron-userland/electron-builder` | `electron/forge` |
| Stars | 14,635 | 7,121 |
| Open issues (excl. PRs) | **41** | **224** |
| Open PRs | 38 | 36 |
| Issues *opened* last 90 days | 72 | 13 |
| Issues *closed* last 90 days | **116** | 35 |
| Commits, last 52 weeks | **442** | 151 |
| Commits, last 12 weeks | **219** | 36 |
| Latest stable release | `electron-builder@26.15.7`, 2026-07-18 | `v7.11.2`, 2026-05-20 |
| Next major in flight | `27.0.0-alpha.6`, 2026-07-24 | `v8.0.0-alpha.10`, 2026-07-02 |
| npm downloads, last month | **13,299,861** (`electron-builder`) | 4,014,431 (`@electron-forge/cli`) |

Sources: `https://api.github.com/repos/electron-userland/electron-builder`,
`https://api.github.com/repos/electron/forge`, the `/releases`, `/stats/participation` and
`/search/issues` endpoints for each, and
`https://api.npmjs.org/downloads/point/last-month/{package}`. For scale, `electron` itself had
20,941,669 downloads in the same window — so roughly two thirds of Electron installs pull
electron-builder.

Both projects are alive; neither is abandoned. But the shapes differ sharply. electron-builder is
closing issues faster than they arrive (116 closed against 72 opened in 90 days) and sits at a
backlog of 41 — remarkable for a project this size, and a deliberate triage outcome. Forge has a
standing backlog of 224 against very low inbound traffic (13 new issues in 90 days) and a commit
log where a large share of recent entries are `dependabot[bot]` dependency bumps.

Bus factor is a real risk on both sides. electron-builder's recent work is concentrated in
`mmaietta` (Mike Maietta) plus a `claude[bot]` automation account; `develar`, the original author,
is historic. Forge's top contributors (`malept`, `MarshallOfSound`) are also largely historic, with
`erickzhao` and `dsanders11` carrying current work inside the Electron org.

### Why electron-builder for this project specifically

Four concrete reasons, none of which are about popularity.

**1. Forge has no AppImage maker and no NSIS maker.** The complete official maker list, taken from
the source tree (`packages/maker/` in `electron/forge`) and corroborated by the published docs
sitemap at `https://www.electronforge.io/sitemap.md`, is: `appx`, `deb`, `dmg`, `flatpak`, `msix`,
`pkg`, `rpm`, `snap`, `squirrel` (Squirrel.Windows), `wix` (MSI), `zip`. That is the whole set.
AppImage — the one Linux format that runs on any distribution without installing anything — is not
in it, and neither is NSIS, the standard Windows installer. Forge's default template
(`packages/template/base/tmpl/forge.config.js`) ships `maker-squirrel`, `maker-zip` (darwin),
`maker-deb` and `maker-rpm` — no DMG, no AppImage, no NSIS.

electron-builder's own target guide calls `squirrel.windows` "Legacy (not recommended)"
(`website/docs/targets.md:16`) and NSIS "the default and best choice for most applications"
(`targets.md:20`).

**2. electron-builder ships Forge makers to plug Forge's gaps — and says not to bother.**
electron-builder publishes `electron-forge-maker-nsis`, `electron-forge-maker-nsis-web`,
`electron-forge-maker-appimage` and `electron-forge-maker-snap`
(`website/docs/features/electron-forge.md:17-22`). The same page carries this note verbatim:

> [Publishing], [Auto Update], and [Code Signing] are only available when using electron-builder as
> your primary build tool. If you need any of those features, migrate fully to electron-builder
> rather than using these makers.
> — `website/docs/features/electron-forge.md:8`

**3. Forge's documentation has no CI or GitHub Actions guide at all.** The full docs sitemap
(`https://www.electronforge.io/sitemap.md`, fetched 2026-08-04) lists getting-started, CLI,
configuration, plugins, makers, publishers, templates, code signing, icons, framework integration,
WSL, auto-update, debugging and extension guides. There is no continuous-integration page;
`https://www.electronforge.io/guides/ci` returns 404. electron-builder has two GitHub Actions pages
(`website/docs/github-actions.md` and `website/docs/features/github-actions.md`) with complete
matrix workflows. For a maintainer who has never used Actions, this asymmetry is decisive.

**4. rpm without a system dependency.** Forge's rpm maker states: "You can only build the RPM
target on Linux machines with the `rpm` or `rpm-build` packages installed"
(`packages/maker/rpm/README.md`). electron-builder builds deb/rpm through a bundled FPM toolset it
downloads on demand (`website/docs/toolsets.md`, `targets.md`), so no host package is needed. In
practice this is a wash on CI — `rpm 4.18.2+dfsg-2.1build2` *is* preinstalled on the `ubuntu-24.04`
image (`actions/runner-images`, `images/ubuntu/Ubuntu2404-Readme.md:307`) — but it matters for local
Fedora builds and it is one less thing to discover the hard way.

### The case *for* Forge, stated fairly

Forge lives inside the `electron` GitHub org and is the only tool Electron's own distribution
overview names: "you can either use specialized tooling like Electron Forge or do it manually"
(`https://www.electronjs.org/docs/latest/tutorial/distribution-overview`). Electron's code-signing
doc calls Forge "the recommended way to sign your app" for Windows
(`electron/electron`, `docs/tutorial/code-signing.md:127`) — but that sentence is about Azure
Artifact Signing, which corerules is not doing. Forge's default template also enables Electron
Fuses and `plugin-auto-unpack-natives` out of the box, which is genuinely good hygiene that
electron-builder leaves to you.

None of that outweighs "no AppImage, no NSIS, no CI documentation" for this project.

### Version to pin

Pin **`electron-builder` 26.x**, explicitly, and do **not** take 27.

There is a live packaging trap: as of 2026-08-04 the npm dist-tags for `electron-builder` are
`latest: 26.15.3`, `v26: 26.15.7`, `next: 27.0.0-alpha.6`
(`https://registry.npmjs.org/electron-builder`). The `latest` tag is stale by four patch releases —
`npm install --save-dev electron-builder` gets you 26.15.3, not 26.15.7. Write the exact version
into `package.json`.

v27 is still alpha and is a large break: native ESM throughout, minimum Node.js 22.12.0, and all
macOS signing options moved from flat `mac.identity` into a `mac.sign` object
(`website/docs/migration/v27-breaking-changes.md:329-341`). It also removes implicit publishing —
"publishing never happens unless you request it" (`website/docs/publish.md:26`). There is an
`electron-builder migrate-schema` command that rewrites config automatically when the time comes.

---

## 2. Artifact formats per OS

### Linux — ship AppImage, rpm, deb

**AppImage** is the primary. Single self-contained file, no installation, no root, runs on
virtually any x86_64 distribution; electron-builder builds it by default
(`website/docs/targets.md:148-153`, `website/docs/appimage.md`). It is also the format least
affected by the Fedora signature change described below.

**rpm** matters because Fedora is the dev machine and Fedora users are the natural audience
(`targets.md:160-162`). **deb** is nearly free once rpm is configured and covers Ubuntu/Mint
(`targets.md:155-158`).

**Skip snap and flatpak.** Both add sandboxing questions (file access to the user's character
directory) and store credentials, and Snap in particular wants `SNAPCRAFT_STORE_CREDENTIALS`
(`website/docs/features/github-actions.md:225`). Neither buys anything for a single-user desktop
tool distributed from GitHub Releases.

One v27 note for later: every Linux target will launch through a generated `<executableName>-launcher`
shell script rather than the binary directly (`website/docs/linux.md:51`). Irrelevant on v26,
relevant if a `.desktop` override is ever hand-written.

### Windows — ship NSIS, optionally portable

**NSIS** (`.exe` installer) is the default and the right choice: it supports per-user install with
no admin rights, which avoids stacking a UAC prompt on top of the SmartScreen prompt
(`targets.md:20-24`).

**portable** (`.exe`, no install) is worth a second artifact for an unsigned build. It skips the
installer entirely — the user downloads one file and runs it. SmartScreen still fires on the
executable, so it does not dodge the warning; it just removes a step (`targets.md:31-35`).

**Skip msi, appx, msix, squirrel.windows.** MSI is for Group Policy/SCCM deployment; AppX/MSIX are
Store/MDM formats that require a trusted certificate for sideloading (`targets.md:37-47`);
Squirrel.Windows is marked legacy by electron-builder's own docs (`targets.md:16`).

### macOS — ship DMG and ZIP, both architectures

**DMG** is the standard consumer format (`targets.md:92-96`). **ZIP** is worth shipping alongside
it, and not only as an auto-update payload: see §3 for why a `.zip` extracted from the command line
is the cleanest escape hatch on an unsigned build.

**Skip pkg** — it needs a separate "Developer ID Installer" certificate to be worth anything
(`targets.md:98-102`).

**Architecture is not optional.** `macos-latest` is now **macOS 26 on arm64**
(`actions/runner-images` README: the `macos-latest` label maps to the macOS 26 Arm64 image). A
default `electron-builder --mac` on that runner produces an **arm64-only** build, and every Intel
Mac user gets nothing. Pass `--mac --x64 --arm64` (two separate artifacts) or build a universal
binary. Two separate artifacts is the simpler choice for v1; universal doubles the download size
and complicates native modules (`website/docs/architecture.md:222-234`).

---

## 3. The unsigned-build user experience, exactly

### macOS — read this section before writing any config

This is where the research turned up something that changes the tooling configuration, not just
the README.

#### Three facts that combine badly

**Fact 1 — Apple Silicon refuses unsigned arm64 code outright.** From Apple's Platform Security
guide, Rosetta 2 section:

> "A Mac with Apple silicon doesn't permit native arm64 code to execute unless a valid signature is
> attached."
> — https://support.apple.com/guide/security/rosetta-2-on-a-mac-with-apple-silicon-secebb113be1/web

The same passage notes the signature may be a bare ad-hoc one from `codesign(1)` carrying no real
identity, and that translated x86_64 code may run with no signature information at all. So: unsigned
x64 runs, unsigned arm64 does not.

**Fact 2 — electron-builder does not ad-hoc sign for you.** Verbatim from its macOS code-signing
doc:

> "If no valid certificate is found, signing is skipped for all architectures — electron-builder
> does **not** apply an ad-hoc signature automatically."
> — `website/docs/features/code-signing/code-signing-mac.md:5`

**Fact 3 — `macos-latest` is arm64.** Established in §2.

Together: a naive `electron-builder --mac` on `macos-latest` with no certificate produces an
unsigned arm64 app, which macOS will not execute at all. Electron's own code-signing documentation
leads with a screenshot captioned exactly that outcome:

> `![macOS Sonoma Gatekeeper warning: The app is damaged](../images/gatekeeper.png)`
> — `electron/electron`, `docs/tutorial/code-signing.md:12`

A user report matching this precisely — DMG built in a GitHub Actions pipeline, unsigned, "damaged"
on the user's machine but fine when built locally — is electron-builder issue
[#8191](https://github.com/electron-userland/electron-builder/issues/8191) (opened 2024-05-02,
closed 2024-05-12). That is a community report, not documentation, and it is offered as
corroboration only.

#### Why "damaged" is much worse than "cannot be verified"

Apple's support article "Safely open apps on your Mac" (published 2026-05-27,
https://support.apple.com/en-us/102445) distinguishes the two cases explicitly. The recoverable one:

> "If the app developer can't be verified and — in macOS Catalina and later — the app hasn't been
> notarized by Apple, macOS can't verify that the app is free of malware."

The dialog for that case reads, per the article's own image alt text:

> "Apple cannot check 'Example App' for malicious software" — with options to "Move to Trash" or
> "Done".

The unrecoverable one, same article:

> "If macOS detects that software has been modified or damaged, your Mac notifies you that the app
> can't be opened. The app might be broken or corrupted, or it might have been tampered with."

The article's "Open Anyway" instructions appear under the heading *"If you want to open an app that
hasn't been notarized or is from an unidentified developer"* — the first case. Apple documents no
override path for the "damaged" case. **Inference, flagged as such:** an app that fails signature
validation entirely does not surface an Open Anyway button, leaving Terminal (`xattr`) as the only
route. I could not test this and Apple does not state it either way.

#### The fix: ad-hoc sign

Set the app to ad-hoc signing. In electron-builder v26 config:

```yaml
mac:
  identity: "-"                      # ad-hoc signature, no Apple identity
  hardenedRuntime: true              # keep it on
  entitlements: build/entitlements.mac.plist
  entitlementsInherit: build/entitlements.mac.plist
```

with at minimum these two entitlements, both of which electron-builder documents as required:

- `com.apple.security.cs.disable-library-validation` — without it the app crashes at launch with
  "`[framework] not valid for use in process: mapping process and mapped file (non-platform) have
  different Team IDs`", because Electron's prebuilt frameworks carry Apple's Team ID and an ad-hoc
  signature does not (`code-signing-mac.md:41-49`).
- `com.apple.security.cs.allow-jit` — "which Electron requires" (`code-signing-mac.md:50`).

This converts the failure mode from "damaged, no way out" to "Apple cannot check…, with a
documented Open Anyway path". In v27 the same keys live under `mac.sign.*`.

**Open conflict, flagged:** electron-builder's `mac.md:102` says an ad-hoc signed app "will only run
on the machine it was built on". Apple's own developer forum guidance says ad-hoc signed code can
generally be copied between machines and still run, with quarantine — not the signature — being the
sticking point. Those cannot both be right in general. This *must* be tested on a real Mac before
v1 ships; see "Things I could not establish".

#### Control-click is gone — the exact click path now

Apple removed the long-standing right-click-Open bypass in macOS 15 Sequoia. Verbatim from Apple's
developer news post *"Updates to runtime protection in macOS Sequoia"*, dated **August 6, 2024**:

> "In macOS Sequoia, users will no longer be able to Control-click to override Gatekeeper when
> opening software that isn't signed correctly or notarized. They'll need to visit System Settings >
> Privacy & Security to review security information for software before allowing it to run."
> — https://developer.apple.com/news/?id=saqachfa

**So the README needs two paths, keyed by macOS version:**

**macOS 15 Sequoia and macOS 26 Tahoe** (current shipping release is macOS 26). From Apple's macOS
User Guide, *"Open an app by overriding security settings"*, version selector set to macOS Tahoe 26
(https://support.apple.com/guide/mac-help/mh40617/mac):

1. Double-click the app once. It will refuse to open. Dismiss the dialog. *(Required — the button
   in step 4 only appears for the app you last tried to launch.)*
2. "On your Mac, choose Apple menu > System Settings, then click Privacy & Security in the sidebar.
   (You may need to scroll down.)"
3. "Go to Security, then click Open."
4. "Click Open Anyway." — the guide notes: "This button is available for about an hour after you try
   to open the app."
5. "Enter your login password, then click OK."

Support article 102445 gives a slightly shorter variant of the same flow (open System Settings →
Privacy & Security → scroll down → Open Anyway → the warning reappears → click Open), after which
"The app is now saved as an exception to your security settings, and you can open it in the future
by double-clicking it, just as you can any authorized app." The two Apple pages differ in step
granularity; the User Guide version above is the more precise one.

**macOS 14 Sonoma and earlier:** right-click (or Control-click) the app in Finder, choose **Open**,
then click **Open** in the dialog. Per Apple's developer news post above, this stopped working in
Sequoia. Note that neither current Apple page mentions Control-click at all any more — the User
Guide page for macOS 26 has no reference to it.

#### `xattr -dr com.apple.quarantine`

Apple does **not** document this as a user step in either article; neither page mentions the
quarantine attribute by name, and the Platform Security Gatekeeper page discusses "provenance"
rather than `com.apple.quarantine`. electron-builder does document it, in its troubleshooting page:

> **"App is damaged and can't be opened"**
> : Usually means the app was downloaded without Gatekeeper quarantine being cleared, and
> notarization failed or wasn't performed. Run `xattr -dr com.apple.quarantine /path/to/app`
> temporarily during testing.
> — `website/docs/troubleshooting.md:190-191`

Treat this as the last-resort instruction in the README, clearly marked as requiring Terminal and
as not Apple-sanctioned. Note that electron-builder frames it as a *testing* aid, not a
distribution instruction.

**Inference, flagged:** a `.zip` extracted with the command-line `unzip` does not propagate the
quarantine attribute to the extracted contents, because `unzip` does not carry extended attributes,
whereas Archive Utility and DMG mounting both do. If true, `unzip corerules-mac.zip` in Terminal is
the cleanest unsigned path on macOS — no Gatekeeper prompt at all. I could not find this stated in
any Apple document and did not test it. It is the reason to ship a ZIP alongside the DMG, but the
README should not promise it until it is verified.

#### What Apple says about the underlying policy

> "By default, macOS Catalina and later also requires software to be notarized, so you can be
> confident that the software you run on your Mac doesn't contain known malware."
> — https://support.apple.com/en-us/102445

Electron's position, for the record:

> "Both Windows and macOS prevent users from running unsigned applications. It is possible to
> distribute applications without codesigning them - but in order to run them, users need to go
> through multiple advanced and manual steps."
> — `electron/electron`, `docs/tutorial/code-signing.md:14-16`

### Windows

#### SmartScreen — what Microsoft actually documents

Microsoft's own SmartScreen overview (last updated 2026-04-23) describes the mechanism but not the
dialog:

> "Checking downloaded files against a list of files that are well known and downloaded frequently.
> If the file isn't on that list, Microsoft Defender SmartScreen shows a warning, advising caution."

> "It also provides reputation checks for apps, checking downloaded programs and the digital
> signature used to sign a file. If a URL, a file, an app, or a certificate has an established
> reputation, users don't see any warnings. If there's no reputation, the item is marked as a higher
> risk and presents a warning to the user."
> — https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/

The click path — the blue "Windows protected your PC" dialog, "More info", then "Run anyway" — is
**not documented on any Microsoft-owned page I could find**. It appears only in Microsoft Q&A and
Tech Community forum threads, which are user-generated. See "Things I could not establish". The
README will have to describe it from a screenshot taken on a real Windows machine rather than cite a
source.

#### Smart App Control — the genuine risk

This is the Windows finding worth flagging. From Microsoft Learn's developer overview (last updated
2025-11-18):

> "Malware, Potentially Unwanted Apps (PUA), and unknown, unsigned code are blocked by default."

> "In *enforcement mode*, Smart App Control is actively protecting your device. Apps cannot be run
> unless they are recognized by Microsoft's app intelligence services, or they are signed with a
> trusted certificate."
> — https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/overview

And from Microsoft's consumer FAQ:

> "If the app is unsigned, or the signature is invalid, Smart App Control will consider it untrusted
> and block it for your protection."

> "There is currently no way to bypass Smart App Control protection for individual apps."
> — https://support.microsoft.com/en-us/windows/security/threat-malware-protection/smart-app-control-frequently-asked-questions

There is no "Run anyway". The user's only options are to turn Smart App Control off entirely or not
run the app.

Three things limit the blast radius. It requires Windows 11 build 22572 or higher. It has
historically required a clean install, though the FAQ now states that "Recent Windows updates allow
Smart App Control to be enabled without requiring a clean installation" — the Learn developer page
still carries the older clean-install-only language, so the two Microsoft sources conflict. And
Microsoft turns it off for people it detects as developers: "If we detect that you're one of those
users, we automatically turn Smart App Control off."

Practical consequence for the README: some Windows 11 users will find corerules simply will not run
and will get no override. The README should tell them how to check
(**Settings → Windows Security → App and Browser Control**, look for a **Smart App Control** section
showing **On** / **Evaluation** / **Off**) and be honest that On means the app cannot be run without
turning the feature off.

#### There is no cheap middle ground on Windows signing

From Electron's code-signing doc:

> "It is important to call out that since June 2023, Microsoft requires software to be signed with an
> 'extended validation' certificate… These simpler certificates no longer provide benefits: Windows
> will treat your app as completely unsigned and display the equivalent warning dialogs."
> — `electron/electron`, `docs/tutorial/code-signing.md:149-154`

EV certificates must live on FIPS 140 Level 2 hardware, so "the certificate cannot be simply
downloaded onto a CI infrastructure" (`code-signing.md:156-158`). The only CI-compatible option is
Azure Artifact Signing, which Electron calls "the cheapest option for code signing on Windows"
(`code-signing.md:105`) — but it is paid, requires an Azure account, and is "currently limited to
developers in certain countries" (`code-signing.md:107`). Out of scope for v1, worth knowing exists.

### Linux

#### AppImage — chmod, and the GUI equivalent

From the AppImage project's own quickstart
(https://docs.appimage.org/introduction/quickstart.html):

Terminal:
1. `cd` to the directory containing the file.
2. "Make the AppImage executable: `chmod +x my.AppImage`"
3. "Run the AppImage: `./my.AppImage`"

GUI — the wording differs by file manager, and the README should give all three:
1. "Right-click on the AppImage and click the 'Properties' entry"
2. "Switch to the Permissions tab"
3. Tick the executable flag:
   - GNOME Files / Nemo / Caja (Nautilus-based): **"Allow executing file as program"**
   - KDE Dolphin: **"Is executable"** checkbox
   - PCManFM: change the **"Execute"** dropdown to **"Anyone"**
4. Close the dialog and double-click the AppImage.

AppImages do not integrate into the desktop menu by themselves — electron-builder's own doc is
explicit that since electron-builder 21 this is not handled by AppImage, and points at
AppImageLauncher (`website/docs/appimage.md:47-57`). The README should say the app will not appear
in the applications menu unless the user installs AppImageLauncher.

#### rpm — fine today, breaking in Fedora 45

Today, an unsigned local rpm installs without complaint. `dnf`'s `localpkg_gpgcheck` — "Whether to
perform a GPG signature check on local packages (packages in a file, not in a repository)" —
defaults to `False` (https://dnf.readthedocs.io/en/latest/conf_ref.html). RPM's own
`%_pkgverify_level` currently defaults to `digest`.

That changes. The Fedora Change *"Enforcing signature checking by default"* is **accepted for
Fedora Linux 45** (categorised `ChangeAcceptedF45`, FESCo #3504, page last updated 2026-01-21):

> "`%_pkgverify_level` default is changed from `digest` to `all`, which requires packages to have
> both a verified signature(s) and digest(s) to be installable."

> "`rpm` will refuse to install such packages, unless explicitly overridden with `--nosignature` (or
> corresponding API)."

> "Packages without verifiable signature(s) cannot be installed without an explicit override."
> — https://fedoraproject.org/wiki/Changes/Enforcing_signature_checking_by_default

The page also records that the change was originally bundled with RPM 6.0 and "was postponed to
Fedora 45 due to time and resource reasons". Fedora 45 is not yet released as of 2026-08-04
(inference from Fedora's six-month cadence: expected around October/November 2026).

**Consequence for corerules.** The unsigned rpm is the Linux artifact with a shelf life. On Fedora 45
and later, `sudo dnf install ./corerules.rpm` will fail for the maintainer's own distribution unless
the user passes an override. This is a strong argument for making **AppImage the documented primary
Linux artifact** and treating rpm/deb as conveniences. It also gives ticket 09 a concrete thing to
document: the `--nosignature` override, or a local `%_pkgverify_level digest` macro, and the fact
that this will start biting Fedora users this autumn.

#### deb

**Inference, flagged:** `dpkg -i` performs no signature verification on standalone `.deb` files at
all — Debian's signature model is on the repository, not the package. So an unsigned deb installs
silently. I did not verify this against Debian's own documentation in this pass.

---

## 4. The GitHub Actions release workflow

### Runner labels, as of 2026-08-04

From `actions/runner-images` README (https://github.com/actions/runner-images):

| Label | Actual image | Arch |
|---|---|---|
| `ubuntu-latest` | Ubuntu 24.04 | x64 |
| `windows-latest` | Windows Server 2025 with VS2026 | x64 |
| `macos-latest` | **macOS 26 Arm64** | **arm64** |
| `macos-26-intel` | macOS 26 | x64 |

The README also warns that `-latest` labels move: "The `-latest` migration process is gradual and
happens over 1-2 months… To avoid unwanted migration, users can specify a specific OS version in the
yaml file (ex: macos-14, windows-2022, ubuntu-22.04)." For a release pipeline that Wagner will touch
rarely, pinning explicit versions is the safer default; the tradeoff is having to bump them by hand.

Preinstalled on `ubuntu-24.04` and relevant here: Node.js 22.23.1 and 24.18.0, `rpm 4.18.2`, `dpkg`,
`fakeroot`, GitHub CLI 2.96.0 (`images/ubuntu/Ubuntu2404-Readme.md`).

### Current action versions

These matter because **electron-builder's own workflow examples are stale** — they show
`actions/checkout@v4`, `actions/setup-node@v4` and `actions/upload-artifact@v4`
(`website/docs/features/github-actions.md:32-53`). Latest releases as of 2026-08-04, from each
action's own repository:

| Action | Latest release | Published |
|---|---|---|
| `actions/checkout` | **v7.0.1** | 2026-07-20 |
| `actions/setup-node` | **v7.0.0** | 2026-07-14 |
| `actions/upload-artifact` | **v7.0.1** | 2026-04-10 |
| `actions/download-artifact` | **v8.0.1** | 2026-03-11 |
| `actions/cache` | **v6.1.0** | 2026-06-26 |
| `softprops/action-gh-release` | v3.0.2 | 2026-07-13 |

### The shape of the workflow

The recommended shape, synthesised from electron-builder's "Complete Production Workflow"
(`website/docs/features/github-actions.md:235-306`) with the action versions and runner
considerations corrected:

```yaml
name: Release

on:
  push:
    tags: ['v*.*.*']
  workflow_dispatch:          # lets Wagner run it by hand from the Actions tab

permissions:
  contents: write             # required to create the Release; see §5

jobs:
  release:
    strategy:
      fail-fast: false        # one platform failing shouldn't kill the others
      matrix:
        include:
          - os: ubuntu-latest
            args: --linux
          - os: windows-latest
            args: --win
          - os: macos-latest
            args: --mac --x64 --arm64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v7
      - uses: actions/setup-node@v7
        with:
          node-version: 22
          cache: npm
      - run: npm ci
      - run: npx electron-builder ${{ matrix.args }} --publish always
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

Notes on each decision:

- **`--publish always`** — "The `--publish always` flag uploads artifacts to a GitHub release for the
  pushed tag. If no release exists, electron-builder creates a draft release."
  (`github-actions.md:104`).
- **`GH_TOKEN`** — electron-builder's env var for the GitHub publisher. Its docs: "Use `GITHUB_TOKEN`
  (provided automatically) or a PAT with `repo` scope" (`github-actions.md:9`). Also: "If `GH_TOKEN`
  or `GITHUB_TOKEN` is defined — defaults to `[{provider: "github"}]`" (`website/docs/publish.md:15`),
  so no `publish:` block is strictly needed in the config.
- **Draft by default.** The GitHub provider's `releaseType` defaults to `draft` — from the source:
  "The type of release. By default `draft` release will be created."
  (`packages/builder-util-runtime/src/publishOptions.ts:134-140`). This is the right behaviour: all
  three runners upload into the same draft, and nothing is public until Wagner clicks Publish. The
  documented completion step is "go to Releases → find the draft → click 'Edit' → 'Publish release'"
  (`github-actions.md:117`).
- **`fail-fast: false`** — not in electron-builder's example; added because with the default,
  a macOS failure cancels the in-flight Linux and Windows jobs and you get a partial draft release.
- **All three jobs write to one draft.** Three parallel jobs creating/uploading to the same draft is
  the documented pattern, but it is a race on first run. If it proves flaky, the alternative is
  `--publish never` + `actions/upload-artifact@v7` in the matrix, then a single dependent job that
  downloads everything and creates the release once. That is the more robust design and worth
  considering in ticket 09.

### Tag-triggered versus `workflow_dispatch`

**Tag-triggered** is the primary. Locally:

```
git tag v0.1.0
git push origin v0.1.0
```

The `on: push: tags:` trigger fires, all three runners build, artifacts land in a draft release.

**`workflow_dispatch`** adds a manual button. Two constraints from GitHub's docs
(`github/docs`, `content/actions/how-tos/manage-workflow-runs/manually-run-a-workflow.md`):

> "To trigger the `workflow_dispatch` event, the workflow must be in the default branch."

> "If you don't see the **Run workflow** button, the workflow file must use the `workflow_dispatch`
> event trigger."

So the workflow file must be merged to `main` before the button appears at all. Include both
triggers from the start.

### A trap worth knowing before ticket 09 designs the pipeline

A Release created using `GITHUB_TOKEN` **will not trigger another workflow**. From GitHub's docs:

> "When you use the repository's `GITHUB_TOKEN` to perform tasks, events triggered by the
> `GITHUB_TOKEN` will not create a new workflow run", with `workflow_dispatch` and
> `repository_dispatch` as the exceptions.
> — `github/docs`, `data/reusables/actions/actions-do-not-trigger-workflows.md`

So a two-stage design — build workflow creates a release, an `on: release` workflow does something
after — will silently never run the second stage. It needs a PAT or a GitHub App token. The
single-workflow shape above avoids the problem entirely.

---

## 5. First-time GitHub Actions configuration — the manual steps

Wagner has never used Actions. This section is written as click paths.

### What is already correct by default

- **Actions is on.** GitHub's docs: "By default, GitHub Actions is enabled on all repositories and
  organizations." Nothing to do for a new personal repository.
- **`GITHUB_TOKEN` exists automatically.** Forge's docs put it plainly: "If you are publishing your
  app with GitHub Actions, the `GITHUB_TOKEN` secret is pre-populated in every workflow."
  (https://www.electronforge.io/config/publishers/github)
- **No PAT is needed.** For creating a Release and uploading assets, `GITHUB_TOKEN` with
  `contents: write` is sufficient — Forge: "the token requires write permissions to your
  repository's contents to create new releases"; electron-builder: "Use `GITHUB_TOKEN` (provided
  automatically) or a PAT with `repo` scope". A PAT is only needed if the release must trigger a
  further workflow (§4) or if publishing to a *different* repository.
- **Artifact and log retention** defaults to 90 days; public repos allow 1–90. Not relevant to
  Releases, which are permanent.

### What is NOT correct by default — and will break the first run

**`GITHUB_TOKEN` is read-only on a new personal repository.** GitHub's docs, on
`managing-github-actions-settings-for-a-repository`: for a repository created in a personal account,
the token "only has read access for the `contents` and `packages` scopes". Publishing a Release with
that token fails.

**The fix, and the preferred one, is the workflow file**, not the settings page:

```yaml
permissions:
  contents: write
```

at the top level of the workflow (already in the §4 example). This is exactly what Forge's own
publisher documentation instructs, with the same two lines. GitHub's own hardening guidance assumes
this works: "It's good security practice to set the default permission for the `GITHUB_TOKEN` to
read access only for repository contents", with permissions raised per-workflow as needed
(https://docs.github.com/en/actions/reference/security/secure-use). The permission model is
described as: "The permissions for the `GITHUB_TOKEN` are initially set to the default setting for
the enterprise, organization, or repository, and are then adjusted based on any configuration within
the workflow file, first at the workflow level and then at the job level."

One gotcha that bites people: **naming any scope sets every unnamed scope to `none`.** From the
workflow syntax reference: "If you specify the access for any of these permissions, all of those
that are not specified are set to `none`." So `contents: write` alone is fine here, but if the
workflow later needs, say, `packages: read`, it must be named too.

**Fallback, if the workflow-level grant does not take effect** (see the caveat in "Things I could
not establish"), change the repository default:

1. Go to `https://github.com/<owner>/corerules`.
2. Click the **Settings** tab in the row of tabs across the top of the repository (Code, Issues,
   Pull requests, … , Settings). If it is not visible, use the **⋯** overflow dropdown at the end of
   that row.
3. In the left sidebar, scroll to the **Code and automation** group and click **Actions**.
4. Under **Actions**, click **General**.
5. Scroll to the **Workflow permissions** section, near the bottom of the page.
6. Select the option granting read *and write* access for all scopes (the first of the two radio
   buttons; the second grants read access to contents and packages only).
7. Click **Save** directly beneath that section. There are several **Save** buttons on this page —
   use the one inside the Workflow permissions box.

The exact radio-button strings are not reproduced in GitHub's documentation; see "Things I could not
establish".

The same section contains a checkbox labelled **Allow GitHub Actions to create and approve pull
requests** — leave it unchecked, it is unrelated.

### Running the workflow by hand

From GitHub's docs (`manually-run-a-workflow.md`), exact steps:

1. Go to `https://github.com/<owner>/corerules`.
2. Click the **Actions** tab.
3. In the left sidebar, click the name of the workflow (it will be **Release**, from the `name:` key
   in the YAML).
4. Above the list of workflow runs, click the **Run workflow** button on the right.
5. Select the **Branch** dropdown and click the branch to run on (`main`).
6. Click **Run workflow**.

If the **Run workflow** button is absent: the workflow file lacks `workflow_dispatch`, or it is not
yet on the default branch.

### Publishing the release

1. Push a tag: `git tag v0.1.0 && git push origin v0.1.0`.
2. Watch the run at **Actions** → **Release** → the newest run. Three jobs should appear, one per
   OS.
3. When all three are green, go to the repository's **Code** tab and click **Releases** in the right
   sidebar (or go to `https://github.com/<owner>/corerules/releases`).
4. The new release will be marked **Draft**. Click it, then click **Edit** (the pencil icon).
5. Check the attached binaries list. Click **Publish release**.

### Where secrets would go, if ever needed

**Settings → Secrets and variables → Actions → New repository secret.** Not needed for an unsigned
build — no `CSC_LINK`, no `WIN_CSC_LINK`, no `APPLE_ID`. Worth knowing the page exists in case
signing is ever added.

### Other things that bite on first run

- **`macos-latest` is arm64.** Covered in §2 and §4. Without `--x64 --arm64`, Intel Macs get nothing
  and nobody notices until a user complains.
- **`-latest` labels move under you.** See §4.
- **Use `npm ci`, not `npm install`.** electron-builder's troubleshooting: "Native modules need to be
  rebuilt for the target Electron version. Ensure `npm ci` (not `npm install`) is used."
  (`github-actions.md:321`).
- **The `latest` npm tag for electron-builder is stale.** §1. Pin the version.

---

## 6. Native modules in this CI matrix

### The cost

**Rebuild per platform is mandatory and not cross-compilable.** Electron's own documentation:
Electron has "a different application binary interface (ABI) from a given Node.js binary", partly
"due to differences such as using Chromium's BoringSSL instead of OpenSSL", so compiled addons must
be rebuilt; "After you upgrade Electron, you usually need to rebuild the modules"
(https://www.electronjs.org/docs/latest/tutorial/using-native-node-modules). electron-builder's
multi-platform page is blunter:

> "If your app has native dependencies, they can only be compiled on the target platform unless
> [prebuild] is used. Most node modules don't provide prebuilt binaries."
> — `website/docs/features/multi-platform-build.md`

So the three-runner matrix stops being a convenience and becomes a requirement. There is no
building-Windows-on-Linux escape hatch: "You cannot build for Windows using Docker if your app has
native dependencies that don't use prebuild" (same page).

### What the tools do for you

Both handle the rebuild automatically. electron-builder: "electron-builder handles this automatically
during the build process when `nativeModules.npmRebuild: true` (the default)"
(`website/docs/architecture.md:216`). Forge: `@electron/rebuild` "is used automatically in both
development mode and when making distributables" (Electron docs, native modules tutorial).

Both also handle ASAR unpacking of `.node` files — electron-builder "automatically detects
executables and native modules and unpacks them from the ASAR" (`website/docs/contents.md:189`);
Forge ships `@electron-forge/plugin-auto-unpack-natives` in its default template.

### The architecture multiplier

Because `macos-latest` is arm64, a native module means:

- **Two separate Mac artifacts** — each arch built and each module compiled for that arch.
- **Or a universal binary**, which "requires that each native module has both x64 and arm64
  variants. If a module only has one architecture, you'll need to use `singleArchFiles`"
  (`website/docs/architecture.md:226-234`).

electron-builder v27 additionally filters `node_modules` by each package's `cpu`/`os` fields against
the target on every build (`architecture.md`, v27 note) — usually helpful, occasionally surprising.

Windows-specific: on Electron 4 and later there is no `node.dll`, so native modules need
`'win_delay_load_hook': 'true'`, and a missing hook shows up as `Module did not self-register`
(Electron native-modules doc).

### The finding that may make all of this moot — `node:sqlite`

**SQLite is built into Electron. It does not require a native module.**

- Electron 43.3.0 (current stable, released 2026-08-04) bundles **Node.js 24.18.1**
  (https://releases.electronjs.org/releases.json).
- `node:sqlite` in Node 24 is **Stability 1.2 — release candidate**, added in v22.5.0, and "SQLite
  is no longer behind `--experimental-sqlite`" as of v23.4.0 / v22.13.0
  (https://nodejs.org/docs/latest-v24.x/api/sqlite.html). The API is `DatabaseSync` with `exec`,
  `prepare`, `run`/`get`/`all`/`iterate`, plus `backup`, sessions/changesets, user-defined functions
  and aggregates.
- Electron explicitly supports it. It broke in Electron 37.2.0 —
  [electron/electron#47671](https://github.com/electron/electron/issues/47671), "`No such binding:
  sqlite`" — and was fixed within a day by
  [#47706](https://github.com/electron/electron/pull/47706): "Fixes an issue where
  `require('node:sqlite')` didn't work — this happened in #47555 via nodejs/node#58122, which added
  the option to build without sqlite. That didn't get correctly copied to the GN build config."
  A regression that was reported and fixed this fast is a regression against a supported feature.

**Consequence for ticket 08.** That ticket's "Packaging cost" bullet reads: "SQLite in Electron means
a native module, rebuilt per platform, which complicates the CI matrix in ticket 09." As of Electron
43 that premise no longer holds. SQLite via `node:sqlite` costs **zero** native modules, zero rebuild
steps, zero ASAR unpacking, and leaves the CI matrix trivially simple. The files-versus-database
decision should be made on the ticket's other four axes — backup and portability, hand-editing and
repair, future sync, querying — not on packaging cost.

Three caveats before that is treated as settled: Stability 1.2 means the API can still change before
1.0; `node:sqlite`'s API is narrower than `better-sqlite3`'s; and I verified support in Electron's
issue tracker, not by running a packaged app. See below.

---

## Things I could not establish

1. **Whether an ad-hoc signed Electron app runs on a machine other than the one that built it.**
   electron-builder's `mac.md:102` says an ad-hoc signature means the "app will only run on the
   machine it was built on"; Apple developer forum guidance (secondary, not documentation) says
   ad-hoc signed code copies between machines fine and quarantine is the real obstacle. These
   contradict. Since the entire recommended macOS configuration in §3 rests on ad-hoc signing, this
   must be tested on a real Mac — build on `macos-latest`, download on a different Mac — before v1
   ships. If electron-builder is right, there is no viable unsigned macOS story at all and macOS
   should be dropped from v1 or shipped ZIP-only.

2. **Whether a fully unsigned app shows the "Open Anyway" button.** Apple documents the Open Anyway
   path only under "an app that hasn't been notarized or is from an unidentified developer". Apple
   documents no override for the "damaged" case. I inferred that no button appears; I did not test
   it and Apple does not say either way.

3. **Whether `unzip` on the command line avoids quarantine propagation on macOS.** Widely believed,
   materially useful (it would give the cleanest unsigned path), and not stated in any Apple
   document I could find. Untested.

4. **The exact SmartScreen dialog wording, from a Microsoft source.** "Windows protected your PC" /
   "Microsoft Defender SmartScreen prevented an unrecognized app from starting. Running this app
   might put your PC at risk." / "More info" / "Run anyway" appears only in Microsoft Q&A and Tech
   Community forum posts — user-generated content on a Microsoft domain, not documentation. The
   README will need a screenshot from a real Windows 11 machine.

5. **The Mark-of-the-Web "Properties → Unblock" checkbox, from a Microsoft source.** Same problem —
   community threads only. Not verified.

6. **The exact radio-button labels under Settings → Actions → General → Workflow permissions.**
   GitHub's docs describe them functionally ("read and write access for all permissions" versus
   "just read access for the `contents` and `packages` permissions") but do not reproduce the
   strings. The commonly reported labels are "Read and write permissions" and "Read repository
   contents and packages permissions" — treat as unverified until seen on the page.

7. **Whether a workflow-level `permissions: contents: write` reliably elevates above a repository
   default of read-only.** The documented model ("initially set to the default… then adjusted based
   on any configuration within the workflow file") and GitHub's own security guidance both imply
   yes, and Forge's publisher docs instruct exactly this. But I did not run it. §5 gives the settings-page
   fallback for this reason. First-run failure with a 403 on release creation is the symptom.

8. **Whether `node:sqlite` works in a *packaged* Electron app.** The evidence (Electron issue #47671,
   fix #47706) comes from Electron Fiddle, i.e. development mode. A packaged, ASAR-bundled app should
   behave identically since no `.node` file is involved, but I did not verify it. Ticket 08 should
   confirm with a five-minute spike before relying on it.

9. **Whether the three matrix jobs racing to create the same draft release is reliable in practice.**
   It is the pattern electron-builder documents, but three parallel `--publish always` runs against a
   non-existent release is a create-or-attach race. §4 sketches the safer upload-artifact-then-release
   alternative; I could not find electron-builder documentation acknowledging the race either way.

10. **`dpkg`'s treatment of unsigned standalone `.deb` files.** Asserted from general knowledge in §3
    and marked as inference. Not checked against Debian documentation in this pass.

11. **Fedora 45's release date.** Inferred from Fedora's six-month cadence as roughly October/November
    2026. The Change page confirms the target release but not a date.

12. **Whether `actions/upload-artifact@v4` is still supported.** v7.0.1 is current; GitHub retired the
    v3 artifact actions in 2025. Whether v4 is deprecated, sunset or merely old was not established —
    moot if the workflow uses v7 as recommended.
