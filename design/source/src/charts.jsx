// Lightweight inline SVG charts for the dashboard

function LineChart({ data, width = 600, height = 180, accent = "var(--accent)" }) {
  const padL = 36, padR = 16, padT = 14, padB = 26;
  const W = width - padL - padR;
  const H = height - padT - padB;
  const max = 100, min = 40;
  const xs = (i) => padL + (W * i) / (data.length - 1);
  const ys = (v) => padT + H - (H * (v - min)) / (max - min);
  const path = data.map((d, i) => `${i ? "L" : "M"}${xs(i).toFixed(1)} ${ys(d.score).toFixed(1)}`).join(" ");
  const area = `${path} L${xs(data.length - 1)} ${padT + H} L${xs(0)} ${padT + H} Z`;
  const gridY = [50, 60, 70, 80, 90];

  return (
    <svg viewBox={`0 0 ${width} ${height}`} width="100%" height={height} style={{ display: "block" }}>
      <defs>
        <linearGradient id="lcfill" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor={accent} stopOpacity="0.18" />
          <stop offset="100%" stopColor={accent} stopOpacity="0" />
        </linearGradient>
      </defs>
      {gridY.map((g) => (
        <g key={g}>
          <line x1={padL} x2={width - padR} y1={ys(g)} y2={ys(g)} stroke="var(--border)" strokeDasharray="2 4" />
          <text x={padL - 8} y={ys(g) + 3} fontSize="10" fill="var(--muted)" fontFamily="var(--mono)" textAnchor="end">{g}</text>
        </g>
      ))}
      <path d={area} fill="url(#lcfill)" />
      <path d={path} fill="none" stroke={accent} strokeWidth="1.8" strokeLinejoin="round" />
      {data.map((d, i) => (
        <g key={i}>
          {(i === data.length - 1 || i === 0) && (
            <circle cx={xs(i)} cy={ys(d.score)} r="3" fill={accent} stroke="var(--surface)" strokeWidth="1.5" />
          )}
          {i % 2 === 0 && (
            <text x={xs(i)} y={height - 8} fontSize="10" fill="var(--muted)" fontFamily="var(--mono)" textAnchor="middle">{d.week}</text>
          )}
        </g>
      ))}
    </svg>
  );
}

function BarChart({ data, width = 600, height = 220, vertical = true, valueKey = "score", labelKey = "subject", max = 100, accent = "var(--accent)" }) {
  const padL = 100, padR = 24, padT = 12, padB = 14;
  const W = width - padL - padR;
  const H = height - padT - padB;
  const barH = (H - (data.length - 1) * 8) / data.length;
  return (
    <svg viewBox={`0 0 ${width} ${height}`} width="100%" height={height} style={{ display: "block" }}>
      {data.map((d, i) => {
        const y = padT + i * (barH + 8);
        const w = (W * d[valueKey]) / max;
        return (
          <g key={i}>
            <text x={padL - 10} y={y + barH / 2 + 4} fontSize="11" fill="var(--text-2)" textAnchor="end">{d[labelKey]}</text>
            <rect x={padL} y={y} width={W} height={barH} fill="var(--surface-2)" />
            <rect x={padL} y={y} width={w} height={barH} fill={accent} fillOpacity="0.85" />
            <text x={padL + w + 6} y={y + barH / 2 + 4} fontSize="11" fontFamily="var(--mono)" fill="var(--text)">{d[valueKey]}</text>
          </g>
        );
      })}
    </svg>
  );
}

function Histogram({ data, width = 600, height = 180, accent = "var(--accent)", highlightBin }) {
  const padL = 24, padR = 16, padT = 12, padB = 30;
  const W = width - padL - padR;
  const H = height - padT - padB;
  const max = Math.max(...data.map((d) => d.count));
  const barW = (W - (data.length - 1) * 8) / data.length;
  return (
    <svg viewBox={`0 0 ${width} ${height}`} width="100%" height={height} style={{ display: "block" }}>
      {data.map((d, i) => {
        const h = (H * d.count) / max;
        const x = padL + i * (barW + 8);
        const y = padT + H - h;
        const isHi = d.bin === highlightBin;
        return (
          <g key={i}>
            <rect x={x} y={y} width={barW} height={h} fill={isHi ? accent : "var(--surface-3)"} stroke={isHi ? accent : "var(--border)"} strokeWidth={isHi ? 0 : 1} />
            <text x={x + barW / 2} y={padT + H + 14} fontSize="10" fill="var(--muted)" fontFamily="var(--mono)" textAnchor="middle">{d.bin}</text>
            <text x={x + barW / 2} y={y - 4} fontSize="10" fill={isHi ? accent : "var(--text-2)"} fontFamily="var(--mono)" textAnchor="middle">{d.count}</text>
          </g>
        );
      })}
    </svg>
  );
}

function ScatterChart({ data, width = 600, height = 240, accent = "var(--accent)" }) {
  const padL = 44, padR = 16, padT = 16, padB = 36;
  const W = width - padL - padR;
  const H = height - padT - padB;
  const xs = (v) => padL + W * v;
  const ys = (v) => padT + H - H * v;
  return (
    <svg viewBox={`0 0 ${width} ${height}`} width="100%" height={height} style={{ display: "block" }}>
      {/* grid */}
      {[0.25, 0.5, 0.75].map((g) => (
        <g key={g}>
          <line x1={xs(g)} x2={xs(g)} y1={padT} y2={padT + H} stroke="var(--border)" strokeDasharray="2 4" />
          <line x1={padL} x2={padL + W} y1={ys(g)} y2={ys(g)} stroke="var(--border)" strokeDasharray="2 4" />
        </g>
      ))}
      <line x1={padL} x2={padL} y1={padT} y2={padT + H} stroke="var(--border)" />
      <line x1={padL} x2={padL + W} y1={padT + H} y2={padT + H} stroke="var(--border)" />

      {/* axis labels */}
      <text x={padL + W / 2} y={height - 8} fontSize="10" fontFamily="var(--mono)" fill="var(--muted)" textAnchor="middle">DIFFICULTY  →</text>
      <text x={12} y={padT + H / 2} fontSize="10" fontFamily="var(--mono)" fill="var(--muted)" textAnchor="middle" transform={`rotate(-90 12 ${padT + H / 2})`}>DISCRIMINATION  →</text>

      {data.map((d, i) => (
        <g key={i}>
          <circle cx={xs(d.difficulty)} cy={ys(d.discrim)} r="6" fill={accent} fillOpacity="0.18" stroke={accent} />
          <text x={xs(d.difficulty) + 9} y={ys(d.discrim) + 3} fontSize="10" fill="var(--text-2)" fontFamily="var(--mono)">{d.q} {d.topic}</text>
        </g>
      ))}
    </svg>
  );
}

function DonutChart({ correct, total, size = 120 }) {
  const r = size / 2 - 10;
  const c = 2 * Math.PI * r;
  const pct = correct / total;
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
      <circle cx={size/2} cy={size/2} r={r} fill="none" stroke="var(--surface-2)" strokeWidth="10" />
      <circle cx={size/2} cy={size/2} r={r} fill="none" stroke="var(--accent)" strokeWidth="10"
        strokeDasharray={`${c * pct} ${c}`}
        transform={`rotate(-90 ${size/2} ${size/2})`}
        strokeLinecap="round" />
      <text x={size/2} y={size/2 - 2} textAnchor="middle" fontFamily="var(--serif)" fontSize="22" fontWeight="500" fill="var(--text)">{Math.round(pct * 100)}%</text>
      <text x={size/2} y={size/2 + 14} textAnchor="middle" fontFamily="var(--mono)" fontSize="9" fill="var(--muted)" letterSpacing="0.8">{correct}/{total}</text>
    </svg>
  );
}

Object.assign(window, { LineChart, BarChart, Histogram, ScatterChart, DonutChart });
