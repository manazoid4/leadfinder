export type EngravingConfig = {
  productImageUrl: string;
  engraving: {
    x: number;
    y: number;
    maxWidth: number;
    colour: [number, number, number];
  };
};

export function createEngravingPreview(
  canvas: HTMLCanvasElement,
  config: EngravingConfig,
): Promise<{ render(text: string): void }>;
