## ✨ Features

- **Download media** from YouTube and many other platforms via [yt-dlp](https://github.com/yt-dlp/yt-dlp).
- **Format options** — choose the output format, and combine or separate audio and video streams.
- **Multimedia processing** — conversion and muxing handled by [FFmpeg](https://ffmpeg.org/).
- **Trim editor** — cut your media with a visual timeline, thumbnail previews, and a waveform display.
- **Live progress tracking** for every download and processing task, with clear status feedback.
- **100% local** — everything runs on your device; the app only provides the interface.

## 🛠️ Technologies Used

- **Frontend:** Svelte
- **Backend / Desktop:** Rust + [Tauri](https://tauri.app/)
- **Multimedia processing:**
  - [yt-dlp](https://github.com/yt-dlp/yt-dlp) (data extraction)
  - [FFmpeg](https://ffmpeg.org/) (conversion, muxing and editing)

## 🚀 Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) and [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/) toolchain
- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [FFmpeg](https://ffmpeg.org/)

### Commands

```bash
pnpm install        # Install dependencies
pnpm dev            # Frontend only (Vite dev server)
pnpm tauri dev      # Full desktop app (development)
pnpm build          # Frontend production build
pnpm tauri build    # Bundle the desktop app
pnpm check          # Typecheck (svelte-check)
```

## ⚖️ License and Legal Notice

### Project License

This project is free software distributed under the **GNU General Public License v3.0 (GPLv3)**. See the `LICENSE` file for more information.

### Third-Party Licenses

You can check the licenses of all third-party tools in [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).

### Legal Disclaimer

- **Intended Purpose:** This app is a portfolio project built with educational intent and designed to help content creators, video editors, and power users. Instead of dealing with command-line tools, it provides a clean, visual interface to combine, trim, and format media quickly for legitimate editing workflows.
- **Educational Purpose:** This application has been developed exclusively for educational purposes, technical research, and as part of a personal portfolio.
- **Local Processing:** The software functions 100% locally on the user's device as a graphical interface (GUI) that executes local operations. The developer provides only the interface and does not host, stream, or distribute any media content.
- **Responsibility:** The developer does not encourage, promote, or take responsibility for the misuse of this tool to download or modify copyright-protected material. It is the sole responsibility of the user to comply with local legislation and the Terms of Service of the source platforms.
- **Ethical Usage & Copyright:** This tool is meant for personal backups, self-owned content, and royalty-free media. The developer does not endorse or condone unauthorized downloads or copyright infringement. Users assume full legal responsibility for how they use this tool in accordance with local laws and platform policies.
