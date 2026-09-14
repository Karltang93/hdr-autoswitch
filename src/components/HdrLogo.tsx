import React from 'react';

interface HdrLogoProps {
  className?: string;
  active?: boolean;
  size?: number;
}

export const HdrLogo: React.FC<HdrLogoProps> = ({
  className = 'w-8 h-8',
  active = false,
  size = 32,
}) => {
  return (
    <div className={`relative inline-flex items-center justify-center ${className}`}>
      <svg
        viewBox="0 0 100 100"
        width={size}
        height={size}
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        className="transition-transform duration-500 hover:scale-110"
      >
        <defs>
          {/* Active HDR Gradients */}
          <linearGradient id="hdrDiamondGrad" x1="10" y1="10" x2="90" y2="90" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="#00f2fe" />
            <stop offset="50%" stopColor="#4facfe" />
            <stop offset="100%" stopColor="#f43f5e" />
          </linearGradient>

          <linearGradient id="sdrDiamondGrad" x1="10" y1="10" x2="90" y2="90" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="#38bdf8" />
            <stop offset="100%" stopColor="#6366f1" />
          </linearGradient>

          <linearGradient id="irisGrad" x1="30" y1="30" x2="70" y2="70" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor={active ? '#f43f5e' : '#38bdf8'} />
            <stop offset="100%" stopColor={active ? '#fbbf24' : '#818cf8'} />
          </linearGradient>

          {/* Neon Glow Filter */}
          <filter id="logoGlow" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="3" result="blur" />
            <feComposite in="SourceGraphic" in2="blur" operator="over" />
          </filter>
        </defs>

        {/* Ambient Glow Rays (Visible when HDR is active) */}
        {active && (
          <g opacity="0.85" filter="url(#logoGlow)">
            {/* Spectral Rays */}
            <path d="M 50 15 L 50 2" stroke="#00f2fe" strokeWidth="2" strokeLinecap="round" opacity="0.9" />
            <path d="M 50 85 L 50 98" stroke="#f43f5e" strokeWidth="2" strokeLinecap="round" opacity="0.9" />
            <path d="M 15 50 L 2 50" stroke="#38bdf8" strokeWidth="2" strokeLinecap="round" opacity="0.9" />
            <path d="M 85 50 L 98 50" stroke="#fbbf24" strokeWidth="2" strokeLinecap="round" opacity="0.9" />
            <path d="M 25 25 L 12 12" stroke="#00f2fe" strokeWidth="1.5" strokeLinecap="round" opacity="0.7" />
            <path d="M 75 25 L 88 12" stroke="#a855f7" strokeWidth="1.5" strokeLinecap="round" opacity="0.7" />
            <path d="M 25 75 L 12 88" stroke="#3b82f6" strokeWidth="1.5" strokeLinecap="round" opacity="0.7" />
            <path d="M 75 75 L 88 88" stroke="#f43f5e" strokeWidth="1.5" strokeLinecap="round" opacity="0.7" />
          </g>
        )}

        {/* Outer Rhombus / Geometric Diamond */}
        <polygon
          points="50,6 94,50 50,94 6,50"
          stroke={active ? 'url(#hdrDiamondGrad)' : 'url(#sdrDiamondGrad)'}
          strokeWidth="2.5"
          fill={active ? 'rgba(15, 23, 42, 0.7)' : 'rgba(15, 23, 42, 0.6)'}
          filter={active ? 'url(#logoGlow)' : undefined}
        />

        {/* Facet Lines (Geometric Prism Cuts) */}
        <line x1="50" y1="6" x2="50" y2="30" stroke={active ? '#00f2fe' : '#64748b'} strokeWidth="1.2" opacity="0.6" />
        <line x1="50" y1="70" x2="50" y2="94" stroke={active ? '#f43f5e' : '#64748b'} strokeWidth="1.2" opacity="0.6" />
        <line x1="6" y1="50" x2="30" y2="50" stroke={active ? '#38bdf8' : '#64748b'} strokeWidth="1.2" opacity="0.6" />
        <line x1="70" y1="50" x2="94" y2="50" stroke={active ? '#fbbf24' : '#64748b'} strokeWidth="1.2" opacity="0.6" />

        {/* Diagonal facets */}
        <line x1="28" y1="28" x2="36" y2="36" stroke={active ? '#38bdf8' : '#64748b'} strokeWidth="1" opacity="0.4" />
        <line x1="72" y1="28" x2="64" y2="36" stroke={active ? '#a855f7' : '#64748b'} strokeWidth="1" opacity="0.4" />
        <line x1="28" y1="72" x2="36" y2="64" stroke={active ? '#38bdf8' : '#64748b'} strokeWidth="1" opacity="0.4" />
        <line x1="72" y1="72" x2="64" y2="64" stroke={active ? '#f43f5e' : '#64748b'} strokeWidth="1" opacity="0.4" />

        {/* Inner Aperture Ring */}
        <circle
          cx="50"
          cy="50"
          r="19"
          stroke={active ? 'url(#hdrDiamondGrad)' : '#94a3b8'}
          strokeWidth="1.8"
          fill="rgba(6, 9, 15, 0.85)"
        />

        {/* Geometric Camera Iris Aperture Blades */}
        <g stroke={active ? 'url(#irisGrad)' : '#94a3b8'} strokeWidth="1.4" strokeLinecap="round">
          <path d="M 50 31 L 59 44" />
          <path d="M 66 40 L 61 56" />
          <path d="M 66 59 L 52 66" />
          <path d="M 50 69 L 41 56" />
          <path d="M 34 60 L 39 44" />
          <path d="M 34 41 L 48 34" />
        </g>

        {/* Core Radiant Sparkle */}
        <circle
          cx="50"
          cy="50"
          r={active ? '5' : '3.5'}
          fill={active ? '#ffffff' : '#38bdf8'}
          filter={active ? 'url(#logoGlow)' : undefined}
          className={active ? 'animate-pulse' : ''}
        />
      </svg>
    </div>
  );
};
