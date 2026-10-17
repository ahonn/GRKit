/** ColorChecker Classic in 8-bit sRGB (X-Rite values after November 2014), dark skin to black. */
const PATCHES = [
  [115, 82, 68], [194, 150, 130], [98, 122, 157], [87, 108, 67], [133, 128, 177], [103, 189, 170],
  [214, 126, 44], [80, 91, 166], [193, 90, 99], [94, 60, 108], [157, 188, 64], [224, 163, 46],
  [56, 61, 150], [70, 148, 73], [175, 54, 60], [231, 199, 31], [187, 86, 149], [8, 133, 161],
  [243, 243, 242], [200, 200, 200], [160, 160, 160], [122, 122, 121], [85, 85, 85], [52, 52, 52],
] as const;

const WIDTH = 1500;
const HEIGHT = 1000;

/** Draws a 3:2 test chart: the 24 patches, a gray ramp, and a hue and lightness sweep. */
export async function drawChart(): Promise<Blob> {
  const canvas = new OffscreenCanvas(WIDTH, HEIGHT);
  const context = canvas.getContext('2d');
  if (!context) {
    throw new Error('2D canvas is not available');
  }
  context.fillStyle = 'rgb(30, 30, 30)';
  context.fillRect(0, 0, WIDTH, HEIGHT);
  const margin = 40;
  const gap = 16;
  const patch = (WIDTH - margin * 2 - gap * 5) / 6;
  PATCHES.forEach(([r, g, b], index) => {
    context.fillStyle = `rgb(${r}, ${g}, ${b})`;
    const x = margin + (index % 6) * (patch + gap);
    const y = margin + Math.floor(index / 6) * (patch * 0.62 + gap);
    context.fillRect(x, y, patch, patch * 0.62);
  });
  const top = margin + 4 * (patch * 0.62 + gap) + 8;
  const ramp = context.createLinearGradient(margin, 0, WIDTH - margin, 0);
  ramp.addColorStop(0, 'rgb(0, 0, 0)');
  ramp.addColorStop(1, 'rgb(255, 255, 255)');
  context.fillStyle = ramp;
  context.fillRect(margin, top, WIDTH - margin * 2, 70);
  const sweepTop = top + 70 + gap;
  const sweepHeight = HEIGHT - margin - sweepTop;
  const hue = context.createLinearGradient(margin, 0, WIDTH - margin, 0);
  for (let i = 0; i <= 12; i++) {
    hue.addColorStop(i / 12, `hsl(${i * 30}, 75%, 50%)`);
  }
  context.fillStyle = hue;
  context.fillRect(margin, sweepTop, WIDTH - margin * 2, sweepHeight);
  const shade = context.createLinearGradient(0, sweepTop, 0, sweepTop + sweepHeight);
  shade.addColorStop(0, 'rgba(255, 255, 255, 0.85)');
  shade.addColorStop(0.5, 'rgba(255, 255, 255, 0)');
  shade.addColorStop(0.5, 'rgba(0, 0, 0, 0)');
  shade.addColorStop(1, 'rgba(0, 0, 0, 0.85)');
  context.fillStyle = shade;
  context.fillRect(margin, sweepTop, WIDTH - margin * 2, sweepHeight);
  return canvas.convertToBlob({ type: 'image/png' });
}
