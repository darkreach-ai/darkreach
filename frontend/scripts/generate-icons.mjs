/**
 * Generate PNG icon thumbnails from the master SVG logo.
 *
 * Produces icons at standard sizes (16–512) for favicons, PWA manifests,
 * and Apple touch icons. Uses sharp for SVG→PNG rasterization.
 *
 * Usage: node scripts/generate-icons.mjs
 */

import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PUBLIC = join(__dirname, "..", "public");
const MASTER_SVG = join(PUBLIC, "logo-master.svg");

/** Standard icon sizes to generate */
const SIZES = [16, 32, 48, 64, 96, 128, 180, 192, 256, 384, 512];

async function generate() {
  const svg = await readFile(MASTER_SVG);

  const results = await Promise.all(
    SIZES.map(async (size) => {
      const name = size === 180 ? "apple-touch-icon.png" : `icon-${size}.png`;
      const out = join(PUBLIC, name);

      await sharp(svg, { density: Math.max(72, Math.round((72 * size) / 512 * 4)) })
        .resize(size, size, { fit: "contain", background: { r: 9, g: 9, b: 11, alpha: 1 } })
        .png({ compressionLevel: 9 })
        .toFile(out);

      return { name, size };
    })
  );

  for (const { name, size } of results) {
    console.log(`  ${name} (${size}x${size})`);
  }
  console.log(`\nGenerated ${results.length} icons from logo-master.svg`);
}

generate().catch((err) => {
  console.error("Icon generation failed:", err.message);
  process.exit(1);
});
