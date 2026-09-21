import { createContext, useContext } from 'react';

export interface ThemeContextType {
  isDark: boolean;
}

export const ThemeContext = createContext<ThemeContextType>({ isDark: true });

export const useTheme = () => useContext(ThemeContext);
