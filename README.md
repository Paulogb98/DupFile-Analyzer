<h1 align="center">DupFile-Analyzer</h1>

<br>
<br>

<p align="center">
  <a href="#-about"><strong>About</strong></a> •
  <a href="#-features"><strong>Features</strong></a> •
  <a href="#-installation"><strong>Installation</strong></a> •
  <a href="#-usage"><strong>Usage</strong></a> •
  <a href="#-algorithms-explained"><strong>Algorithms</strong></a> •
  <a href="#-how-its-fast"><strong>Performance</strong></a> •
  <a href="#-architecture"><strong>Architecture</strong></a> •
  <a href="#-license"><strong>License</strong></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.75%2B-orange?style=flat-square&logo=rust" alt="Rust 1.75+" />
  <img src="https://img.shields.io/github/stars/Paulogb98/DupFile-Analyzer?style=flat-square&logo=github" alt="GitHub stars" />
  <img src="https://img.shields.io/github/issues/Paulogb98/DupFile-Analyzer?style=flat-square&logo=github" alt="GitHub issues" />
  <img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License" />
</p>

<br>

## 📖 About

**DupFile-Analyzer** is a high-performance, **interactive** command-line tool written in
Rust that finds duplicate files — and *visually similar images* — inside a directory and its
subdirectories.

Run it with no arguments and it greets you with a friendly terminal wizard: pick what to
scan with the arrow keys, choose an algorithm, point it at a folder, and it does the rest.
It detects **exact duplicates** by content hash (SHA-256) and **near-duplicate images** by
**perceptual hashing** (aHash / dHash / pHash) — so it catches resized, recompressed, or
lightly edited copies that a byte-for-byte hash would miss.

> Identify duplicates by content — and images by how they *look*. Fast, safe, interactive.

<br>

## ✨ Features

| Feature | Description |
|---|---|
| **Interactive wizard** | Arrow-key menus greet you on launch — no flags to memorize |
| **Exact duplicate detection** | SHA-256 content hashing for absolute, byte-level accuracy |
| **Perceptual image matching** | aHash, dHash & pHash find *visually similar* images |
| **Parallel processing** | Multi-threaded hashing via Rayon across all CPU cores |
| **Size pre-filter** | Files with a unique size skip hashing entirely — big speedup |
| **Two-stage hashing** | Cheap partial hash before the full SHA-256 short-circuits large reads |
| **Wasted-space summary** | Reports exactly how much disk you can reclaim |
| **Scan filters** | Min/max size, exclude globs, hidden-file & symlink toggles |
| **Export** | Save results as JSON, CSV, or plain text |
| **Safe deletion** | Optionally delete or move duplicates — dry-run first, always confirmed |

<br>

## 🌍 Cross-Platform Support

| Platform | Support |
|----------|---------|
| **Windows 10/11** | ✅ Native |
| **Linux** | ✅ Native |
| **macOS** | ✅ Native (Intel & Apple Silicon) |

<br>

## ⚙️ Requirements

- **Rust** 1.75 or newer ([install here](https://www.rust-lang.org/tools/install)) — includes Cargo.
- A terminal that supports interactive input (any modern terminal does).

<br>

## 🚀 Installation

### Option 1 — Install with Cargo (recommended)

```bash
git clone https://github.com/Paulogb98/DupFile-Analyzer.git
cd DupFile-Analyzer
cargo install --path .
```

Then launch it from anywhere:

```bash
dupfile-analyzer
```

### Option 2 — Build from source

```bash
git clone https://github.com/Paulogb98/DupFile-Analyzer.git
cd DupFile-Analyzer
cargo build --release
./target/release/dupfile-analyzer   # dupfile-analyzer.exe on Windows
```

<br>

## 📖 Usage

DupFile-Analyzer is **fully interactive** — just run it inside (or point it at) the folder
you want to analyze:

```bash
dupfile-analyzer
```

The wizard walks you through every step:

**1. Choose what to scan** (↑/↓ + Enter):

```
? What do you want to scan?
❯ General files — exact duplicates (SHA-256)
  Images — exact duplicates (SHA-256)
  Images — visually similar (perceptual hash)
```

**2. Point it at a directory** (defaults to your current folder).

**3. For image similarity, pick an algorithm and threshold:**

```
? Which perceptual algorithm?
  aHash (average hash) — Fastest. Compares each pixel to the average brightness.
❯ dHash (difference hash) — Balanced. Encodes brightness gradients between neighbors.
  pHash (DCT hash) — Most robust. Uses the DCT low-frequency signature.

? Similarity threshold (max Hamming distance, 0-64): 10
```

**4. Optionally set filters** — min/max size, exclude patterns, hidden files, symlinks.

**5. Read the report**, then choose a **post-scan action**: export the results, or delete /
move the duplicates.

### Example output

```
✔ 1742 file(s) to analyze.

  ⠹ [00:00:12] [========================================] 1742/1742 (100%)

ℹ Found 2 duplicate group(s).

Hash: da0c30d2…523a1  (4.20 MB each)
  - /media/photos/vacation_1.jpg
  - /media/photos/backup/vacation_1.jpg

Hash: e3b0c442…52b855  (0 B each)
  - /projects/app/__init__.py
  - /projects/lib/__init__.py

▸ 2 duplicate file(s) across 2 group(s) — 4.20 MB reclaimable.
```

<br>

## 🧠 Algorithms Explained

DupFile-Analyzer offers four ways to decide whether two files are "the same". Which one to
use depends on whether you want **identical bytes** or **similar-looking images**.

### SHA-256 — exact content match

A cryptographic hash of the file's entire contents. Two files are duplicates **only if every
byte is identical**. Renaming, moving, or changing metadata doesn't matter — only the content
does. Use this for documents, archives, videos, or any file where you want *certainty*.

- ✅ Zero false positives — a match is a guaranteed duplicate.
- ✅ Works on any file type.
- ❌ A single changed byte (re-encoding, resizing an image) makes files look completely different.

### Perceptual hashes — "does it *look* the same?"

Perceptual hashes reduce an image to a small 64-bit fingerprint that changes *gradually* with
the image. Similar images get similar fingerprints, so two pictures are considered
near-duplicates when the **Hamming distance** (number of differing bits) between their hashes
is below a threshold you choose. Lower threshold = stricter; higher = looser.

These catch **resized, recompressed, watermarked, or lightly edited** copies that SHA-256
would never group together.

| Algorithm | How it works | Strengths | Trade-offs |
|-----------|--------------|-----------|------------|
| **aHash** (average hash) | Shrinks the image to 8×8 grayscale, then sets each bit where a pixel is brighter than the overall average. | Fastest, dead simple. | Sensitive to overall brightness/contrast shifts. |
| **dHash** (difference hash) | Shrinks to 9×8 grayscale and sets each bit where a pixel is brighter than its right-hand neighbor — encoding *gradients* rather than absolute brightness. | Great balance of speed and robustness; the default. | Slightly weaker than pHash on heavy edits. |
| **pHash** (DCT hash) | Shrinks to 32×32, applies a 2D Discrete Cosine Transform, and hashes the low-frequency block against its median. | Most robust to scaling, compression, and small edits. | Highest compute cost (the DCT). |

**Rule of thumb:** start with **dHash** at threshold 10. If you're missing obvious matches,
raise the threshold or switch to **pHash**. If you only need to catch trivial resizes at top
speed, **aHash** is enough.

> ⚠️ Perceptual matches are *similar*, not identical — so DupFile-Analyzer never auto-deletes
> perceptual groups. Deletion is offered only for exact (SHA-256) duplicates.

<br>

## ⚡ How It's Fast

Exact scanning never blindly hashes every file. It runs three cheap-to-expensive stages so
most files are eliminated before a full hash is ever computed:

```
All files
   │  1. Group by byte size — a unique size can't have a duplicate
   ▼
Size collisions only
   │  2. Partial hash (first 8 KB) — weeds out same-size non-matches cheaply
   ▼
Partial-hash collisions only
   │  3. Full SHA-256 — confirms true duplicates
   ▼
Duplicate groups
```

On top of that, every stage runs in **parallel across all CPU cores** via Rayon, files are
read through a **64 KB buffer**, and the release build uses fat LTO + `opt-level = 3`.

<br>

## 🏗️ Architecture

```
Launch (interactive wizard)
        │
        ▼
┌──────────────────────┐     ┌──────────────────────────────┐
│  app.rs (wizard)     │────▶│  config.rs (ScanConfig)      │
│  dialoguer menus     │     │  mode · algorithm · filters  │
└──────────┬───────────┘     └───────────────┬──────────────┘
           │                                 │
           ▼                                 ▼
┌──────────────────────────────────────────────────────────┐
│  scanner.rs                                                │
│  walk + filter → size pre-filter → parallel hashing        │
│  exact: partial → full SHA-256    similar: perceptual+UF   │
└──────────┬───────────────────────────────────┬───────────┘
           │                                     │
           ▼                                     ▼
┌──────────────────────┐            ┌──────────────────────────┐
│  report.rs           │            │  export.rs / dedupe.rs   │
│  groups + summary    │            │  JSON·CSV·txt / delete   │
└──────────────────────┘            └──────────────────────────┘
```

### Folder structure

```
DupFile-Analyzer/
├── Cargo.toml
├── LICENSE
├── README.md
├── src/
│   ├── main.rs            # thin entry point → dupfile_analyzer::run()
│   ├── lib.rs             # crate root, module wiring, run()
│   ├── app.rs             # interactive wizard (banner + arrow-key menus)
│   ├── config.rs          # ScanConfig, ScanMode, glob building
│   ├── error.rs           # AppError enum
│   ├── scanner.rs         # walk, filter, size pre-filter, hashing, clustering
│   ├── report.rs          # human-readable report + wasted-space summary
│   ├── export.rs          # JSON / CSV / text export
│   ├── dedupe.rs          # safe delete / move of exact duplicates
│   └── hashing/
│       ├── mod.rs         # Algorithm enum, image-extension helpers
│       ├── content.rs     # SHA-256 full + partial hashing
│       └── perceptual.rs  # aHash / dHash / pHash + Hamming distance
└── tests/
    └── scan.rs            # integration tests (exact, filters, perceptual)
```

<br>

## 🧪 Technical Details

- **Buffered I/O** — files are streamed through a 64 KB buffer; whole files are never held in memory.
- **Parallelism** — Rayon's work-stealing pool hashes files concurrently with a live progress bar.
- **Perceptual clustering** — a union-find (disjoint-set) structure groups every pair of
  images within the Hamming threshold into a single cluster.
- **Wasted space** — computed as `Σ (group_size − 1) × file_size` over exact groups.
- **Release profile** (`Cargo.toml`): `opt-level = 3`, fat LTO, `codegen-units = 1`, stripped binary.

<br>

## 🛠️ Troubleshooting

| Problem | Solution |
|---------|----------|
| `'…' is not a valid directory` | Check the path exists; quote paths with spaces. |
| `Interactive prompt failed: not a terminal` | Run it in a real interactive terminal (not a pipe/CI). |
| A resized image isn't detected | Raise the similarity threshold or switch from aHash/dHash to **pHash**. |
| Empty files reported as duplicates | Expected — all empty files share one SHA-256; keep one, delete the rest. |
| Permission denied on some files | Those files are reported and skipped; re-run with sufficient permissions. |

<br>

## 🚀 Roadmap

### ✅ v1.0 (current)
- ✅ Interactive terminal wizard (arrow-key navigation)
- ✅ Perceptual image hashing (aHash / dHash / pHash)
- ✅ Size pre-filter + two-stage exact hashing
- ✅ Wasted-space summary, scan filters, JSON/CSV/text export
- ✅ Safe, dry-run-first deletion / relocation of duplicates
- ✅ Modular `lib.rs` architecture + integration tests

### 💭 Future
- 💭 Side-by-side thumbnail preview for perceptual matches
- 💭 Hard-link / symlink deduplication (reclaim space without deleting)
- 💭 Resumable scans and an on-disk hash cache
- 💭 Additional hashes (BLAKE3) and content-defined chunking

<br>

## 🤝 Contributing

```bash
git checkout -b feature/YourFeature
cargo test          # run the integration tests
cargo clippy        # keep it warning-free
git commit -m 'feat: add YourFeature'
git push origin feature/YourFeature
```

<br>

## 📄 License

MIT — see the [LICENSE](LICENSE) file.

<br>

## 🙏 Acknowledgments

- 📦 Crates: `walkdir`, `rayon`, `sha2`, `image`, `dialoguer`, `console`, `indicatif`, `globset`, `serde`, `thiserror`
- ✨ The Rust community and ecosystem
