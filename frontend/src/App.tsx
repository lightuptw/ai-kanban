import React, { useEffect } from "react";
import { useRoutes } from "react-router-dom";
import { Provider } from "react-redux";
import { HelmetProvider, Helmet } from "react-helmet-async";
import { CacheProvider } from "@emotion/react";

import { ThemeProvider as MuiThemeProvider } from "@mui/material/styles";
import { AdapterDateFns } from "@mui/x-date-pickers/AdapterDateFns";
import { LocalizationProvider } from "@mui/x-date-pickers/LocalizationProvider";

import "./i18n";
import createTheme from "./theme";
import routes from "./routes";

import useTheme from "./hooks/useTheme";
import { store } from "./redux/store";
import createEmotionCache from "./utils/createEmotionCache";
import { SSEManager } from "./services/sse";

const clientSideEmotionCache = createEmotionCache();

let sseManager: SSEManager | null = null;

function App({ emotionCache = clientSideEmotionCache }) {
  const content = useRoutes(routes);

  const { theme } = useTheme();

  useEffect(() => {
    if (!sseManager) {
      sseManager = new SSEManager(store.dispatch);
      sseManager.connect();
    }

    return () => {
      if (sseManager) {
        sseManager.disconnect();
        sseManager = null;
      }
    };
  }, []);

  return (
    <CacheProvider value={emotionCache}>
      <HelmetProvider>
        <Helmet
titleTemplate="%s | LightUp AI Kanban"
        defaultTitle="LightUp AI Kanban"
        />
        <Provider store={store}>
          <LocalizationProvider dateAdapter={AdapterDateFns}>
            <MuiThemeProvider theme={createTheme(theme)}>
              {content}
            </MuiThemeProvider>
          </LocalizationProvider>
        </Provider>
      </HelmetProvider>
    </CacheProvider>
  );
}

export default App;
