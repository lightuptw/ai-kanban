import React, { useEffect, useState } from "react";
import styled from "@emotion/styled";
import { Outlet, useLocation } from "react-router-dom";

import { Box, CssBaseline, Paper as MuiPaper } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import useMediaQuery from "@mui/material/useMediaQuery";
import { spacing } from "@mui/system";

import GlobalStyle from "../components/GlobalStyle";
import Navbar from "../components/navbar/Navbar";
import dashboardItems from "../components/sidebar/dashboardItems";
import Sidebar from "../components/sidebar/Sidebar";
import Footer from "../components/Footer";
import Settings from "../components/Settings";
import NotificationSnackbar from "../components/NotificationSnackbar";
import UserProfileDialog from "../components/UserProfileDialog";
import { requestNotificationPermission } from "../utils/browserNotifications";
import { useAuth } from "../hooks/useAuth";

const drawerWidth = 258;

const Root = styled.div`
  display: flex;
  min-height: 100vh;
`;

const Drawer = styled.div`
  ${(props) => props.theme.breakpoints.up("md")} {
    width: ${drawerWidth}px;
    flex-shrink: 0;
  }
`;

const AppContent = styled.div`
  flex: 1;
  display: flex;
  flex-direction: column;
  max-width: 100%;
`;

const Paper = styled(MuiPaper)(spacing);

const MainContent = styled(Paper)`
  flex: 1;
  background: ${(props) => props.theme.palette.background.default};

  @media all and (-ms-high-contrast: none), (-ms-high-contrast: active) {
    flex: none;
  }

  .MuiPaper-root .MuiPaper-root {
    box-shadow: none;
  }
`;

interface DashboardType {
  children?: React.ReactNode;
}

const Dashboard: React.FC<DashboardType> = ({ children }) => {
  const router = useLocation();
  const [mobileOpen, setMobileOpen] = useState(false);
  const [profileOpen, setProfileOpen] = useState(false);
  const [onboardingMode, setOnboardingMode] = useState(false);
  const { user } = useAuth();

  const handleDrawerToggle = () => {
    setMobileOpen(!mobileOpen);
  };

  useEffect(() => {
    setMobileOpen(false);
  }, [router.pathname]);

  useEffect(() => {
    requestNotificationPermission();
  }, []);

  // Auto-open profile dialog in onboarding mode for new users
  useEffect(() => {
    if (user && user.profile_completed === false && !profileOpen) {
      setOnboardingMode(true);
      setProfileOpen(true);
    }
  }, [user, profileOpen]);

  const theme = useTheme();
  const isLgUp = useMediaQuery(theme.breakpoints.up("lg"));

  return (
    <Root>
      <CssBaseline />
      <GlobalStyle />
      <Drawer>
        <Box sx={{ display: { xs: "block", lg: "none" } }}>
          <Sidebar
            PaperProps={{ style: { width: drawerWidth } }}
            variant="temporary"
            open={mobileOpen}
            onClose={handleDrawerToggle}
            items={dashboardItems}
            onProfileClick={() => { setOnboardingMode(false); setProfileOpen(true); }}
          />
        </Box>
        <Box sx={{ display: { xs: "none", md: "block" } }}>
          <Sidebar
            PaperProps={{ style: { width: drawerWidth } }}
            items={dashboardItems}
            onProfileClick={() => { setOnboardingMode(false); setProfileOpen(true); }}
          />
        </Box>
      </Drawer>
      <AppContent>
        <Navbar onDrawerToggle={handleDrawerToggle} />
        <MainContent p={isLgUp ? 4 : 2}>
          {children}
          <Outlet />
        </MainContent>
        <Footer />
      </AppContent>
      <Settings />
      <UserProfileDialog
        open={profileOpen}
        onClose={() => { setProfileOpen(false); setOnboardingMode(false); }}
        onboardingMode={onboardingMode}
      />
      <NotificationSnackbar />
    </Root>
  );
};

export default Dashboard;
