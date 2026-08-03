/* eslint-disable @next/next/no-img-element -- brand SVG variants are selected by CSS. */

interface BrandMarkProps {
  size?: number;
}

export function BrandMark({ size = 32 }: BrandMarkProps) {
  return (
    <div data-testid="brand-mark" className="flex items-center gap-3">
      <img
        src="/ame-icon-light.svg"
        alt=""
        aria-hidden="true"
        data-testid="brand-mascot"
        className="brand-mark-light block shrink-0 rounded-lg"
        style={{ width: size, height: size }}
      />
      <img
        src="/ame-icon-dark.svg"
        alt=""
        aria-hidden="true"
        className="brand-mark-dark hidden shrink-0 rounded-lg"
        style={{ width: size, height: size }}
      />
      <span
        data-testid="brand-wordmark"
        className="font-extrabold leading-none tracking-[-0.02em] text-foreground"
        style={{ fontSize: Math.round(size * 0.75) }}
      >
        ame
      </span>
    </div>
  );
}
