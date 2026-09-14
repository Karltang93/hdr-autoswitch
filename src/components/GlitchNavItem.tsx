import React, { useEffect, useRef, useId } from 'react';
import { gsap } from 'gsap';
import { RoughEase } from 'gsap/EasePack';

gsap.registerPlugin(RoughEase);

interface GlitchNavItemProps {
  label: string;
  count?: number;
  isActive: boolean;
  onClick: () => void;
  width?: number;
  height?: number;
}

export const GlitchNavItem: React.FC<GlitchNavItemProps> = ({
  label,
  count,
  isActive,
  onClick,
  width = 160,
  height = 38,
}) => {
  const uniqueId = useId().replace(/[:]/g, '');
  const filterId = `displace-${uniqueId}`;
  const patternId = `scanline-${uniqueId}`;
  const fadeId = `fade-${uniqueId}`;

  const containerRef = useRef<SVGSVGElement | null>(null);
  const redTextRef = useRef<SVGTextElement | null>(null);
  const blueTextRef = useRef<SVGTextElement | null>(null);
  const displaceRef = useRef<SVGFEDisplacementMapElement | null>(null);
  const scanlineRef = useRef<SVGRectElement | null>(null);
  const scanlinePatternRef = useRef<SVGPatternElement | null>(null);
  const fillBlueRef = useRef<SVGRectElement | null>(null);
  const fillRedRef = useRef<SVGRectElement | null>(null);

  const timelinesRef = useRef<{
    text: gsap.core.Timeline;
    active: gsap.core.Timeline;
    scanline: gsap.core.Timeline;
    scanlinePattern: gsap.core.Timeline;
    displacement: gsap.core.Timeline;
    displacementActive: gsap.core.Timeline;
  } | null>(null);

  useEffect(() => {
    if (
      !containerRef.current ||
      !redTextRef.current ||
      !blueTextRef.current ||
      !displaceRef.current ||
      !scanlineRef.current ||
      !scanlinePatternRef.current ||
      !fillBlueRef.current ||
      !fillRedRef.current
    ) {
      return;
    }

    const duration = 0.3;
    const roughEase = RoughEase.ease.config({ strength: 4, points: 8 });

    const timeline = (opts = {}) => gsap.timeline({ paused: true, ...opts });

    const timelines = {
      text: timeline(),
      active: timeline(),
      scanline: timeline(),
      scanlinePattern: timeline(),
      displacement: timeline(),
      displacementActive: timeline({
        repeat: -1,
        repeatDelay: 2.5,
        repeatRefresh: true,
      }),
    };

    // Text slight shift
    timelines.text
      .to(redTextRef.current, { x: 3, duration, ease: roughEase })
      .to(blueTextRef.current, { x: -3, duration, ease: roughEase }, '<0.05');

    // Displacement glitch on hover
    timelines.displacement
      .to(displaceRef.current, {
        attr: { scale: 1.5 },
        duration: 0.12,
        ease: roughEase,
      })
      .to(displaceRef.current, {
        attr: { scale: 0 },
        duration: 0.15,
        ease: 'power2.out',
      });

    // Active repeating jitter
    timelines.displacementActive
      .to(displaceRef.current, {
        attr: { scale: () => Math.random() * 0.4 + 0.1 },
        duration: 0.2,
        ease: roughEase,
      })
      .to(displaceRef.current, {
        attr: { scale: 0 },
        duration: 0.1,
      });

    // Active indicator fills left edge
    timelines.active
      .to(fillBlueRef.current, { attr: { height }, duration: 0.2, ease: 'power2.out' })
      .to(fillRedRef.current, { attr: { height }, duration: 0.2, ease: 'power2.out' }, '<0.05');

    // Scanline animation
    timelines.scanline
      .to(scanlineRef.current, { fill: '#521d20', duration: 0.2 }, 0)
      .to(scanlineRef.current, { opacity: 0.3, duration: 0.1, repeat: -1, yoyo: true }, 0);

    timelines.scanlinePattern.to(scanlinePatternRef.current, {
      attr: { y: 10 },
      duration: 0.65,
      ease: 'none',
      repeat: -1,
    });

    timelinesRef.current = timelines;

    return () => {
      timelines.text.kill();
      timelines.active.kill();
      timelines.scanline.kill();
      timelines.scanlinePattern.kill();
      timelines.displacement.kill();
      timelines.displacementActive.kill();
    };
  }, [width, height]);

  // Update active state animation
  useEffect(() => {
    const tl = timelinesRef.current;
    if (!tl) return;

    if (isActive) {
      tl.active.timeScale(1).play();
      tl.displacementActive.timeScale(1).play();
    } else {
      tl.active.timeScale(1.5).reverse();
      tl.displacementActive.pause(0);
      if (displaceRef.current) {
        gsap.set(displaceRef.current, { attr: { scale: 0 } });
      }
    }
  }, [isActive]);

  const handleMouseEnter = () => {
    const tl = timelinesRef.current;
    if (!tl) return;
    tl.text.timeScale(1).play();
    tl.displacement.timeScale(1).restart();
    tl.scanline.timeScale(1).play();
    tl.scanlinePattern.timeScale(1).play();
  };

  const handleMouseLeave = () => {
    const tl = timelinesRef.current;
    if (!tl) return;
    tl.text.timeScale(2).reverse();
    tl.displacement.timeScale(2).reverse();
    if (!isActive) {
      tl.scanline.pause(0);
      tl.scanlinePattern.pause(0);
    }
  };

  const fullText = count !== undefined ? `${label} (${count})` : label;

  return (
    <button
      onClick={onClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      className="relative block cursor-pointer select-none focus:outline-none transition-transform active:scale-[0.98]"
      style={{ width, height }}
    >
      <svg
        ref={containerRef}
        width="100%"
        height="100%"
        viewBox={`0 0 ${width} ${height}`}
        className="overflow-visible block"
      >
        <defs>
          <filter id={filterId} primitiveUnits="objectBoundingBox" colorInterpolationFilters="sRGB">
            <feFlood floodColor="#8888FF" y="0%" height="5%" result="s0" />
            <feFlood floodColor="#888800" y="5%" height="5%" result="s1" />
            <feFlood floodColor="#8888FF" y="10%" height="5%" result="s2" />
            <feFlood floodColor="#888800" y="15%" height="5%" result="s3" />
            <feFlood floodColor="#8888FF" y="20%" height="5%" result="s4" />
            <feFlood floodColor="#888800" y="25%" height="5%" result="s5" />
            <feFlood floodColor="#8888FF" y="30%" height="5%" result="s6" />
            <feFlood floodColor="#888800" y="35%" height="5%" result="s7" />
            <feFlood floodColor="#8888FF" y="40%" height="5%" result="s8" />
            <feFlood floodColor="#888800" y="45%" height="5%" result="s9" />
            <feFlood floodColor="#8888FF" y="50%" height="5%" result="s10" />
            <feFlood floodColor="#888800" y="55%" height="5%" result="s11" />
            <feFlood floodColor="#8888FF" y="60%" height="5%" result="s12" />
            <feFlood floodColor="#888800" y="65%" height="5%" result="s13" />
            <feFlood floodColor="#8888FF" y="70%" height="5%" result="s14" />
            <feFlood floodColor="#888800" y="75%" height="5%" result="s15" />
            <feFlood floodColor="#8888FF" y="80%" height="5%" result="s16" />
            <feFlood floodColor="#888800" y="85%" height="5%" result="s17" />
            <feFlood floodColor="#8888FF" y="90%" height="5%" result="s18" />
            <feFlood floodColor="#888800" y="95%" height="5%" result="s19" />

            <feMerge result="bands">
              <feMergeNode in="s0" />
              <feMergeNode in="s1" />
              <feMergeNode in="s2" />
              <feMergeNode in="s3" />
              <feMergeNode in="s4" />
              <feMergeNode in="s5" />
              <feMergeNode in="s6" />
              <feMergeNode in="s7" />
              <feMergeNode in="s8" />
              <feMergeNode in="s9" />
              <feMergeNode in="s10" />
              <feMergeNode in="s11" />
              <feMergeNode in="s12" />
              <feMergeNode in="s13" />
              <feMergeNode in="s14" />
              <feMergeNode in="s15" />
              <feMergeNode in="s16" />
              <feMergeNode in="s17" />
              <feMergeNode in="s18" />
              <feMergeNode in="s19" />
            </feMerge>

            <feDisplacementMap
              ref={displaceRef}
              className="displace"
              in="SourceGraphic"
              in2="bands"
              scale="0"
              xChannelSelector="B"
              yChannelSelector="R"
            />
          </filter>

          <pattern
            ref={scanlinePatternRef}
            id={patternId}
            className="scanline-pattern"
            patternUnits="userSpaceOnUse"
            width="5"
            height="10"
          >
            <rect ref={scanlineRef} className="scanline" x="0" y="0" width="5" height="1" fill="#221314" />
            <rect x="0" y="1" width="5" height="9" fill="#0f0b0b" />
          </pattern>

          <radialGradient
            id={fadeId}
            gradientUnits="userSpaceOnUse"
            cx={width / 2}
            cy={height / 2}
            r="40"
            gradientTransform={`translate(${width / 2} ${height / 2}) scale(${width / 40} 1) translate(-${width / 2} -${height / 2})`}
          >
            <stop offset="0%" stopColor="#0f0b0b" stopOpacity="0" />
            <stop offset="60%" stopColor="#0f0b0b" stopOpacity="0" />
            <stop offset="100%" stopColor="#0f0b0b" stopOpacity="0.8" />
          </radialGradient>
        </defs>

        {/* Scanline background */}
        <rect x="0" y="0" width="100%" height={height} fill={`url(#${patternId})`} />
        <rect x="0" y="0" width="100%" height={height} fill={`url(#${fadeId})`} />

        {/* Displaced text elements (red + blue for chromatic aberration) */}
        <g filter={`url(#${filterId})`}>
          <text
            ref={blueTextRef}
            x={width / 2}
            y={height / 2 + 1}
            fill="#5accf5"
            fontSize="12"
            fontWeight="700"
            fontFamily="'Kode Mono', monospace"
            textAnchor="middle"
            dominantBaseline="middle"
            letterSpacing="0.05em"
            opacity={isActive ? 0.9 : 0.6}
          >
            {fullText}
          </text>
          <text
            ref={redTextRef}
            x={width / 2}
            y={height / 2 + 1}
            fill={isActive ? '#ffffff' : '#f55a6b'}
            fontSize="12"
            fontWeight="700"
            fontFamily="'Kode Mono', monospace"
            textAnchor="middle"
            dominantBaseline="middle"
            letterSpacing="0.05em"
          >
            {fullText}
          </text>
        </g>

        {/* Active side indicator stripes */}
        <rect
          ref={fillBlueRef}
          x="0"
          y="0"
          width="4"
          height="0"
          fill="#5accf5"
          className="fill"
          filter={`url(#${filterId})`}
        />
        <rect
          ref={fillRedRef}
          x="0"
          y="0"
          width="4"
          height="0"
          fill="#f55a6b"
          className="fill"
          filter={`url(#${filterId})`}
        />

        {/* Outer border */}
        <rect
          x="0"
          y="0"
          width="100%"
          height={height}
          fill="none"
          stroke={isActive ? '#f55a6b' : 'rgba(245, 90, 107, 0.4)'}
          strokeWidth={isActive ? '2' : '1'}
        />
      </svg>
    </button>
  );
};
