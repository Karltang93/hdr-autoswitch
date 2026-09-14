import React, { useState } from 'react';
import { GlitchText } from './GlitchText';

interface GlitchButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  variant?: 'primary' | 'outline' | 'cyan' | 'ghost';
  icon?: React.ReactNode;
  size?: 'sm' | 'md' | 'lg';
}

export const GlitchButton: React.FC<GlitchButtonProps> = ({
  label,
  variant = 'primary',
  icon,
  size = 'md',
  className = '',
  onClick,
  disabled,
  ...props
}) => {
  const [isHovered, setIsHovered] = useState(false);

  const sizeClasses = {
    sm: 'px-2.5 py-1 text-xs gap-1.5',
    md: 'px-4 py-2 text-sm gap-2',
    lg: 'px-6 py-3 text-base gap-2.5',
  }[size];

  const variantStyles = {
    primary:
      'bg-[#f55a6b] text-[#0f0b0b] font-bold border-2 border-[#f55a6b] hover:bg-[#ff6e7e] hover:shadow-[0_0_20px_rgba(245,90,107,0.6)] active:scale-[0.98]',
    outline:
      'bg-[#0f0b0b] text-[#f55a6b] font-bold border border-[#f55a6b]/60 hover:border-[#f55a6b] hover:bg-[#221314]/60 hover:shadow-[0_0_15px_rgba(245,90,107,0.35)] active:scale-[0.98]',
    cyan:
      'bg-[#5accf5] text-[#0f0b0b] font-bold border-2 border-[#5accf5] hover:bg-[#70d6f7] hover:shadow-[0_0_20px_rgba(90,204,245,0.6)] active:scale-[0.98]',
    ghost:
      'bg-transparent text-[#e5e0e1] border border-white/10 hover:border-[#f55a6b]/50 hover:text-[#f55a6b] hover:bg-[#221314]/30',
  }[variant];

  return (
    <button
      onClick={onClick}
      disabled={disabled}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={`relative inline-flex items-center justify-center cursor-pointer select-none transition-all duration-150 uppercase font-mono tracking-wider disabled:opacity-40 disabled:cursor-not-allowed ${sizeClasses} ${variantStyles} ${className}`}
      {...props}
    >
      {/* Subtle Scanline Overlay inside outline/ghost buttons */}
      {(variant === 'outline' || variant === 'ghost') && (
        <span className="absolute inset-0 scanlines-overlay opacity-30 pointer-events-none" />
      )}

      {/* Left indicator notch */}
      {variant === 'outline' && isHovered && (
        <span className="absolute left-0 top-0 bottom-0 w-1 bg-[#5accf5]" />
      )}

      {icon && <span className="relative z-10">{icon}</span>}
      <GlitchText
        text={label}
        scrambleOnHover={!disabled}
        chromaOnHover={variant !== 'primary'}
        className="relative z-10"
      />
    </button>
  );
};
