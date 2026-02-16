export const THEMES = {
  DEFAULT: "DEFAULT",
  DARK: "DARK",
  LIGHT: "LIGHT",
  BLUE: "BLUE",
  GREEN: "GREEN",
  INDIGO: "INDIGO",
};

export const API_BASE_URL =
  import.meta.env.VITE_API_URL ||
  `${window.location.protocol}//${window.location.hostname}:21547`;
