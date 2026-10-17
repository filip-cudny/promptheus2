import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = join(import.meta.dirname, "..");
const iconsDir = join(root, "src-tauri", "icons");
const flameHeightRatio = 0.8;

const tmp = mkdtempSync(join(tmpdir(), "promptheus-icons-"));

function render(svgPath, outName, extraArgs = []) {
  const out = join(tmp, outName);
  execFileSync("pnpm", ["tauri", "icon", svgPath, "-o", out, ...extraArgs], {
    cwd: root,
    stdio: ["ignore", "ignore", "pipe"],
  });
  return out;
}

function renderPng(svgText, name, size, dest) {
  const svgPath = join(tmp, `${name}.svg`);
  writeFileSync(svgPath, svgText);
  copyFileSync(join(render(svgPath, name, ["--png", String(size)]), `${size}x${size}.png`), dest);
}

function sortIcnsChunks(file) {
  const data = readFileSync(file);
  const chunks = [];
  for (let offset = 8; offset < data.length; ) {
    const length = data.readUInt32BE(offset + 4);
    chunks.push(data.subarray(offset, offset + length));
    offset += length;
  }
  chunks.sort((a, b) => a.compare(b, 0, 4, 0, 4));
  writeFileSync(file, Buffer.concat([data.subarray(0, 8), ...chunks]));
}

function flameSquareSvg(svg) {
  const rim = [...svg.matchAll(/<polygon [^>]*fill="none"[^>]*\/>/g)].at(-1);
  const flame = svg.replace(rim[0], "");
  const points = [...flame.matchAll(/<polygon points="([^"]+)"/g)].flatMap((m) =>
    m[1].trim().split(/\s+/).map((p) => p.split(",").map(Number)),
  );
  const xs = points.map(([x]) => x);
  const ys = points.map(([, y]) => y);
  const [minX, maxX] = [Math.min(...xs), Math.max(...xs)];
  const [minY, maxY] = [Math.min(...ys), Math.max(...ys)];
  const side = (maxY - minY) / flameHeightRatio;
  const x = (minX + maxX) / 2 - side / 2;
  const y = (minY + maxY) / 2 - side / 2;
  return flame.replace(
    /<svg [^>]*>/,
    (tag) =>
      tag
        .replace(/viewBox="[^"]*"/, `viewBox="${x.toFixed(2)} ${y.toFixed(2)} ${side.toFixed(2)} ${side.toFixed(2)}"`)
        .replace(/width="[^"]*"/, `width="${side.toFixed(2)}"`)
        .replace(/height="[^"]*"/, `height="${side.toFixed(2)}"`),
  );
}

try {
  const logo = readFileSync(join(iconsDir, "logo.svg"), "utf8");
  const symbolic = readFileSync(join(iconsDir, "logo-symbolic.svg"), "utf8");

  const app = render(join(iconsDir, "logo.svg"), "app");
  for (const file of ["32x32.png", "128x128.png", "128x128@2x.png", "icon.icns", "icon.ico"]) {
    copyFileSync(join(app, file), join(iconsDir, file));
  }
  sortIcnsChunks(join(iconsDir, "icon.icns"));

  renderPng(symbolic, "tray-macos", 36, join(iconsDir, "tray-macos.png"));
  renderPng(symbolic.replaceAll("#000000", "#ffffff"), "tray-linux", 32, join(iconsDir, "tray-linux.png"));

  const assets = join(iconsDir, "Promptheus.icon", "Assets");
  mkdirSync(assets, { recursive: true });
  renderPng(flameSquareSvg(logo), "flame", 1024, join(assets, "flame.png"));
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
