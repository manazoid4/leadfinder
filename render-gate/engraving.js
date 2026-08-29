// Directional Sobel relief adapted from Konva src/filters/Emboss.ts (MIT).
// Copyright Konva contributors. Ported without the scene-graph dependency.
const clamp8 = (value) => Math.max(0, Math.min(255, value));

export function applyDirectionalEmboss(imageData, maskData, options = {}) {
  const { width, height, data } = imageData;
  const source = new Uint8ClampedArray(data);
  const mask = maskData.data;
  const luminance = new Float32Array(width * height);
  const strength = Math.max(0, Math.min(1, options.strength ?? 0.9));
  const direction = ((options.direction ?? 315) * Math.PI) / 180;
  const colour = options.colour ?? [55, 34, 22];
  const ink = Math.max(0, Math.min(0.5, options.ink ?? 0.2));
  for (let pixel = 0, offset = 0; offset < mask.length; offset += 4, pixel += 1) {
    luminance[pixel] =
      (0.2126 * mask[offset] + 0.7152 * mask[offset + 1] + 0.0722 * mask[offset + 2]) *
      (mask[offset + 3] / 255);
  }
  const gx = [-1, 0, 1, -2, 0, 2, -1, 0, 1];
  const gy = [-1, -2, -1, 0, 0, 0, 1, 2, 1];
  const offsets = [-width - 1, -width, -width + 1, -1, 0, 1, width - 1, width, width + 1];
  const cosine = Math.cos(direction);
  const sine = Math.sin(direction);
  for (let y = 1; y < height - 1; y += 1) {
    for (let x = 1; x < width - 1; x += 1) {
      const pixel = y * width + x;
      let horizontal = 0;
      let vertical = 0;
      for (let index = 0; index < 9; index += 1) {
        horizontal += luminance[pixel + offsets[index]] * gx[index];
        vertical += luminance[pixel + offsets[index]] * gy[index];
      }
      const offset = pixel * 4;
      const maskAlpha = mask[offset + 3] / 255;
      const relief = ((cosine * horizontal + sine * vertical) / 1020) * 88 * strength;
      const stain = maskAlpha * ink;
      for (let channel = 0; channel < 3; channel += 1) {
        const engraved = source[offset + channel] * (1 - stain) + colour[channel] * stain;
        data[offset + channel] = clamp8(engraved - relief);
      }
      data[offset + 3] = source[offset + 3];
    }
  }
  return imageData;
}

function fitFont(context, text, maxWidth, startSize) {
  let size = startSize;
  while (size > 18) {
    context.font = `600 ${size}px Georgia, serif`;
    if (context.measureText(text).width <= maxWidth) return size;
    size -= 2;
  }
  return size;
}

export async function createEngravingPreview(canvas, config) {
  const image = new Image();
  image.crossOrigin = 'anonymous';
  const sourceUrl = new URL(config.productImageUrl);
  sourceUrl.searchParams.set('width', '1280');
  image.src = sourceUrl.toString();
  await image.decode();

  const scale = Math.min(1, 1280 / image.naturalWidth);
  canvas.width = Math.round(image.naturalWidth * scale);
  canvas.height = Math.round(image.naturalHeight * scale);
  const context = canvas.getContext('2d', { willReadFrequently: true });
  const maskCanvas = document.createElement('canvas');
  maskCanvas.width = canvas.width;
  maskCanvas.height = canvas.height;
  const maskContext = maskCanvas.getContext('2d', { willReadFrequently: true });

  return {
    render(text) {
      context.clearRect(0, 0, canvas.width, canvas.height);
      context.drawImage(image, 0, 0, canvas.width, canvas.height);
      if (!text.trim()) return;
      maskContext.clearRect(0, 0, canvas.width, canvas.height);
      const maxWidth = canvas.width * config.engraving.maxWidth;
      const fontSize = fitFont(maskContext, text.trim(), maxWidth, canvas.width * 0.06);
      maskContext.font = `600 ${fontSize}px Georgia, serif`;
      maskContext.textAlign = 'center';
      maskContext.textBaseline = 'middle';
      maskContext.fillStyle = '#ffffff';
      maskContext.fillText(
        text.trim(),
        canvas.width * config.engraving.x,
        canvas.height * config.engraving.y,
        maxWidth,
      );
      const imageData = context.getImageData(0, 0, canvas.width, canvas.height);
      const maskData = maskContext.getImageData(0, 0, canvas.width, canvas.height);
      context.putImageData(
        applyDirectionalEmboss(imageData, maskData, {
          strength: 0.95,
          direction: 315,
          colour: config.engraving.colour,
          ink: 0.24,
        }),
        0,
        0,
      );
    },
  };
}
