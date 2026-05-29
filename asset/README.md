# Portfolio

Personal portfolio built with Vue 3 + Vite. Deployed on GitHub Pages.

## Quick Start

```bash
npm install
npm run dev
```

## Commands

| Command | Description |
|---------|-------------|
| `npm run dev` | Start dev server |
| `npm run build` | Build for production |
| `npm test` | Run all tests |
| `npm run test:watch` | Run tests in watch mode |

## Adding a Project

Edit `src/data/projects.json` and add a new entry:

```json
{
  "id": "my-new-project",
  "name": "My New Project",
  "summary": "Short description",
  "description": "Full Markdown description...",
  "techStack": ["Tech1", "Tech2"],
  "images": [{ "src": "/images/screenshot.png", "alt": "Screenshot" }],
  "links": { "github": "https://...", "demo": null },
  "featured": false
}
```

Place images in `public/images/`.
