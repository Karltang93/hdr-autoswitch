import React, { useState, useRef, useEffect } from 'react';

interface GlitchTextProps {
  text: string;
  className?: string;
  scrambleOnHover?: boolean;
  chromaOnHover?: boolean;
  onClick?: () => void;
  as?: 'span' | 'div' | 'h1' | 'h2' | 'h3' | 'p';
}

const GLITCH_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+-=[]{}|;:<>?';

export const GlitchText: React.FC<GlitchTextProps> = ({
  text,
  className = '',
  scrambleOnHover = true,
  chromaOnHover = true,
  onClick,
  as: Component = 'span',
}) => {
  const [displayText, setDisplayText] = useState(text);
  const [isGlitching, setIsGlitching] = useState(false);
  const intervalRef = useRef<number | null>(null);

  useEffect(() => {
    setDisplayText(text);
  }, [text]);

  const startGlitch = () => {
    if (!scrambleOnHover || isGlitching) return;
    setIsGlitching(true);

    let iteration = 0;
    const maxIterations = text.length;

    if (intervalRef.current) clearInterval(intervalRef.current);

    intervalRef.current = window.setInterval(() => {
      setDisplayText(
        text
          .split('')
          .map((char, index) => {
            if (char === ' ') return ' ';
            if (index < iteration) return text[index];
            return GLITCH_CHARS[Math.floor(Math.random() * GLITCH_CHARS.length)];
          })
          .join('')
      );

      if (iteration >= maxIterations) {
        if (intervalRef.current) clearInterval(intervalRef.current);
        setDisplayText(text);
        setIsGlitching(false);
      }

      iteration += 1 / 2;
    }, 25);
  };

  const stopGlitch = () => {
    if (intervalRef.current) clearInterval(intervalRef.current);
    setDisplayText(text);
    setIsGlitching(false);
  };

  return (
    <Component
      onClick={onClick}
      onMouseEnter={startGlitch}
      onMouseLeave={stopGlitch}
      className={`font-mono transition-colors select-none ${
        chromaOnHover && isGlitching ? 'glitch-text-chroma' : ''
      } ${className}`}
      style={
        isGlitching && chromaOnHover
          ? {
              textShadow: '-2px 0 #5accf5, 2px 0 #f55a6b',
            }
          : undefined
      }
    >
      {displayText}
    </Component>
  );
};
