# SPU reverb, final PCM and a sound diagnostic

`SpuDevice::clock_sample` now connects the voice engine to reverb, main gain
and signed stereo PCM output at 44.1 kHz. Reverb retains its shared sound RAM,
alternating channel steps, mirrored filter rings, wrap behavior, coefficient
edge cases and write-enable semantics. Main gain advances after the sample
has been mixed and saturated. Host playback/resampling remains a separate layer.

`load_spu_reverb_presets` reads ten stock configurations from the identified
local executable. Their zero-depth register values/order match original SDK
recordings with a supplied empty allocator. This data loader does not implement
the SDK's allocation checks, work-area clearing or callback timing.

```sh
cargo test -p lsd-assets --test spu_output --locked
cargo run -p lsd-assets --bin lsd-sound -- /path/SLPS_015.56 /path/AMBIENT.VH 0 output.wav 1
```

`lsd-sound` is a controlled diagnostic using the sound-object adapter, SDK
updates and complete PCM path. The packed note selects `program * 16 + tone`
from the sound object's logical table. The optional final mode selects one of
the ten presets; mode zero is the default. A further optional trace filename
records the supplied register schedule for independent replay.

The command creates a new two-second, 16-bit stereo WAV, with supplied 60 Hz
SDK ticks and release after one second. It uses the original startup master
volume setting and retains the tone's reverb routing. Existing output files
are preserved. These timings/placement are diagnostic inputs, not acceptance
of natural game startup or callback cadence. Original inputs and rendered
audio are not bundled with the repository.

A local ambient-note diagnostic produced 88,200 stereo frames. Replaying its
identified sample body and register schedule through the pinned independent
DSP produced byte-for-byte identical PCM. The WAV header and payload were also
read back independently; this verifies the saved diagnostic, not live playback.

## Comparisons

| Supplied software comparison | Coverage |
| --- | ---: |
| Owned coefficient, RAM-wrap, enable, mix and device scenarios | 83 |
| Owned output-only / complete-device clocks | 46,080 / 9,552 |
| Original SDK preset recordings / register writes each | 10 / 35 |
| Long original-preset response clocks | 2,205,000 |
| Original tone bindings / complete-device clocks | 351 / 718,848 |
| Complete state words checked per observation | 2,101 |

Original-code recordings execute `SpuSetReverbModeParam` and `SsSetMVol` from
the canonical executable with supplied allocator/register storage. The private
DSP oracle uses pinned
[Beetle PSX source](https://github.com/libretro/beetle-psx-libretro/blob/e57860d82faa953dc0fc10f201fd583739bc2f7d/mednafen/psx/spu.c).
Its reverb region is unchanged. The full update has only observation assignments
added to retain wet and PCM values before the emulator's host gain; removing
those assignments restores the original function exactly. The GPL comparison
source stays separate from the MIT implementation.

The reference applies `(sample * 3 + 2) >> 2` for playback headroom. Tests compare
the PCM before that gain, its unchanged post-gain result and wet output. The
Rust SPU exposes the unattenuated PCM; a gain difference cannot conceal a
different raw result. Comparisons also retain voice/register state, reverb
rings/cursors, IRQ latch and final sound RAM. Each stock preset gets a five-second
response trace; each original tone gets a supplied 2,048-clock scenario.

The [hardware reference](https://psx-spx.consoledev.net/soundprocessingunitspu/)
informs the register/DSP model. Software-reference agreement is not physical
hardware or natural original-scene acceptance. See
[spu-output-checks.json](spu-output-checks.json) for identified inputs and checks.

## Remaining integration

The running Bevy scene still queues audio requests. Live output-device and
resampling integration, real bank allocation/transfer, original callback and
IRQ/bus timing, music sequencing and natural scene comparisons remain open.
Reserved ADPCM encodings and disputed high-register pitch behavior retain the
earlier limits. No complete audio group or whole-game parity is accepted yet.
Original behavior remains separate from optional AI tools/mods.
