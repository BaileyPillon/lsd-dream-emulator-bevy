# Data mods

The dream prototype accepts explicitly selected **data replacement packs**.
Texture, map, geometry, model/animation-library and sound-bank replacements use
the existing native-format decoders. A streamed scene and its link destinations
retain the same immutable pack snapshot.

Mods are an optional port feature. They do not change the recovered original NPC
scripts, invoke AI services, or count towards original-game parity acceptance.
Full game flow remains [in development](STATUS.md).

## Install and play

Keep packs outside your original extraction. Pass each directory explicitly,
in order from lowest to highest priority:

```sh
cargo run -p lsd-assets --bin lsd-mod-check --locked -- "/games/lsd/files" --mod "/mods/alternate-textures"
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- "/games/lsd/files" 0 0 A --dream --mod "/mods/alternate-textures"
```

Repeat `--mod <directory>` to combine packs. Options may appear before or after
the existing host options. Quote paths containing spaces. There is no automatic
mod discovery or download. The checker opens no window and writes no files.

With no `--mod` options the original-only mode is used. You can make that choice
explicit:

```sh
cargo run -p lsd-bevy --bin lsd-stage --features desktop --locked -- "/games/lsd/files" 0 0 A --dream --original
```

`--original` and `--mod` together are an error. Restart without pack arguments to
disable mods; the original files are never patched. Modified runs show
**[MODDED]** in the window title and print a JSON `content_profile` at startup.

## Author a pack

A pack directory contains `mod.json` and your replacement files. For example:

```text
alternate-textures/
  mod.json
  textures/
    custom.TIX
```

```json
{
  "schema_version": 1,
  "id": "alternate-textures",
  "name": "Alternate textures",
  "version": "1.0.0",
  "reference_sha256": "282308ade239ce61b5603651dbbab0476a681b5d3618e82c0d3c3a95316d302f",
  "requires": [],
  "conflicts": [],
  "files": [
    {
      "target": "CDI/STG00/TEXA.TIX",
      "source": "textures/custom.TIX",
      "sha256": "REPLACE_WITH_YOUR_FILE_SHA256"
    }
  ]
}
```

Replace the placeholder with the lowercase SHA-256 of your payload. On
PowerShell:

```powershell
(Get-FileHash -Algorithm SHA256 -LiteralPath './alternate-textures/textures/custom.TIX').Hash.ToLowerInvariant()
```

On Linux use `sha256sum alternate-textures/textures/custom.TIX`. Hashes describe
the exact bytes; update the manifest when editing a file. All paths are relative.
The target is the original content path; the source is relative to the pack.
Target matching ignores ASCII case and accepts either slash convention.

Supported targets are under `CDI/`, with one of these extensions:

| Extension | Current consumer |
| --- | --- |
| `.TIX`, `.TIM` | Stage and generation textures |
| `.LBD` | Map geometry, collision and embedded scene content |
| `.TMD` | Player and generation models |
| `.MOM` | Special encounter model/animation library |
| `.VH`, `.VB` | Shared scene sound-bank header and sample data |
| `.SEQ` | Reserved data replacement; live dream music integration is still pending |

The original catalogue still selects resource names. This API replaces resources
that are requested by the prototype; it does not add catalogue entries or new
NPC code. Existing geometry/design-scale and decoder constraints still apply.
The checker validates manifests, paths, compatibility, bounds and hashes;
format decoding happens when a consumer loads the resource. A successfully
checked pack can still contain invalid or unsuitable native-format data.

## Load order and compatibility

- A later pack wins when several packs replace the same target. The startup
  receipt lists the winner and every displaced pack in order.
- `requires` lists pack IDs that must appear earlier. There is no implicit
  activation, sorting, installation, or version-range resolution.
- `conflicts` lists incompatible pack IDs. A conflict declared by either pack
  rejects the profile regardless of order.
- IDs are unique, lowercase ASCII letters/digits plus `-`, `_` and `.`, at most
  64 characters. Names and version labels must be nonempty.
- Schema 1 targets the identified Japanese executable. Startup validates that
  executable independently of the pack declarations. Packs cannot replace it.
- Unknown fields, duplicate targets within a pack, missing payloads, hash
  mismatches, escaping symlinks and nonportable/traversing paths are errors.
- Limits are 32 packs, 128 replacement entries per pack, 64 KiB per manifest,
  16 MiB per file and 64 MiB across all declared payloads, including displaced
  files. No pack is partially activated after a validation failure.

Replacement bytes are snapshotted into shared memory at startup. Editing a pack
during play takes effect on the next launch. No hot reload, native plugin,
script execution, workshop service or AI gameplay integration is implemented.

## Reproduce and report

Save the JSON emitted by `lsd-mod-check`, along with the port commit and normal
scenario inputs. The receipt records the ordered IDs, names, versions, raw
manifest hashes, winning target hashes/sizes, override history and a profile
hash. It contains no local paths or original asset payloads. The profile hash
is SHA-256 of its compact JSON with `profile_sha256` set to the empty string.

The receipt identifies mod inputs; it does not hash every original base asset
or prove a complete deterministic gameplay run. Report base-asset identities,
controller history, timing and starting state separately. Retest original-game
differences with `--original` and use the
[compatibility report](templates/COMPATIBILITY_REPORT.md).

For local integration checks:

```sh
cargo test -p lsd-assets --test mod_profiles --locked
# With your original local files:
LSD_GAME_FILES=/games/lsd/files cargo test -p lsd-assets --test mod_profiles --locked -- --include-ignored
```

Public tests use owned data. The opt-in local test checks a real stage texture
replacement, original-only isolation, immutable snapshots, and destination
reconstruction. It does not accept full rendering or whole-game parity.

Keep original game assets and private AI credentials out of published packs.
AI-assisted authoring remains a [separate tooling track](../tools/ai-assist/README.md).
