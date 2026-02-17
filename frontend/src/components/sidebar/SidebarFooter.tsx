import React from "react";
import styled from "@emotion/styled";

import { Badge, Grid, Avatar, Typography } from "@mui/material";
import { API_BASE_URL } from "../../constants";
import { useAuth } from "../../hooks/useAuth";

const Footer = styled.div`
  background-color: ${(props) =>
    props.theme.sidebar.footer.background} !important;
  padding: ${(props) => props.theme.spacing(2.75)}
    ${(props) => props.theme.spacing(4)};
  border-right: 1px solid rgba(0, 0, 0, 0.12);
  cursor: pointer;
  transition: background-color 150ms ease;

  &:hover {
    background-color: rgba(0, 0, 0, 0.12) !important;
  }
`;

const FooterText = styled(Typography)`
  color: ${(props) => props.theme.sidebar.footer.color};
`;

const FooterSubText = styled(Typography)`
  color: ${(props) => props.theme.sidebar.footer.color};
  font-size: 0.7rem;
  display: block;
  padding: 1px;
`;

const FooterBadge = styled(Badge)`
  margin-right: ${(props) => props.theme.spacing(1)};
  span {
    background-color: ${(props) =>
      props.theme.sidebar.footer.online.background};
    border: 1.5px solid ${(props) => props.theme.palette.common.white};
    height: 12px;
    width: 12px;
    border-radius: 50%;
  }
`;

type SidebarFooterProps = {
  onClick?: () => void;
};

const SidebarFooter: React.FC<SidebarFooterProps> = ({ onClick, ...rest }) => {
  const { user } = useAuth();

  return (
    <Footer onClick={onClick} {...rest}>
      <Grid container spacing={2}>
        <Grid item>
          <FooterBadge
            overlap="circular"
            anchorOrigin={{
              vertical: "bottom",
              horizontal: "right",
            }}
            variant="dot"
          >
            <Avatar
              alt="User"
              src={user?.avatar_url ? `${API_BASE_URL}${user.avatar_url}` : undefined}
            />
          </FooterBadge>
        </Grid>
        <Grid item>
          <FooterText variant="body2">{user?.nickname || "User"}</FooterText>
          <FooterSubText variant="caption">LightUp AI Kanban</FooterSubText>
        </Grid>
      </Grid>
    </Footer>
  );
};

export default SidebarFooter;
