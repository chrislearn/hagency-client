# Website project location

## Hagency website

- Managed path: `projects/hagency-website/`.
- Model: new, independently owned source tree with its own Git repository;
  not a symlink and not a synchronized copy of another checkout.
- Absolute path: `/Users/yuechen/home/hagency/projects/hagency-website`.
- Source remote: `https://github.com/hagency-org/hagency-website.git`.
- Public site: `https://hagency-org.github.io/hagency-website/` (GitHub Pages).
- Stack: Astro, TypeScript, static English/Simplified Chinese routes.
- Preview: `http://127.0.0.1:4328/en/`, `http://127.0.0.1:4328/zh-cn/`.
- Setup and verification: `projects/hagency-website/README.md`.
- Active implementation contract:
  `projects/hagency-website/specs/task-public-website.spec.md`.

The containing directory is also the existing Hagency checkout. Website code
and tests belong exclusively in the nested website repository. The provisioned
`task-writer` wrapper is absent in this checkout; this file records project
location only and is not a replacement canonical task-state source.
