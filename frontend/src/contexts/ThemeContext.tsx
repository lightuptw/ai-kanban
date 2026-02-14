import React, { useEffect } from "react";

import { THEMES } from "../constants";
import { api } from "../services/api";

const initialState = {
  theme: THEMES.DEFAULT,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  setTheme: (theme: string) => {},
};
const ThemeContext = React.createContext(initialState);

type ThemeProviderProps = {
  children: React.ReactNode;
};

function ThemeProvider({ children }: ThemeProviderProps) {
  const [theme, _setTheme] = React.useState<string>(initialState.theme);

  useEffect(() => {
    const storedTheme = localStorage.getItem("theme");
    if (storedTheme) {
      _setTheme(JSON.parse(storedTheme));
    }

    api
      .getSetting("theme")
      .then((setting) => {
        _setTheme(setting.value);
        localStorage.setItem("theme", JSON.stringify(setting.value));
      })
      .catch(() => {});
  }, []);

  const setTheme = (theme: string) => {
    localStorage.setItem("theme", JSON.stringify(theme));
    _setTheme(theme);
    api.setSetting("theme", theme).catch(() => {});
  };

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export { ThemeProvider, ThemeContext };
