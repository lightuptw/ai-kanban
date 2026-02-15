import React from "react";
import styled from "@emotion/styled";
import { withTheme } from "@emotion/react";
import { useSelector, useDispatch } from "react-redux";

import {
  Grid,
  AppBar as MuiAppBar,
  IconButton as MuiIconButton,
  Toolbar,
  Typography,
} from "@mui/material";

import { Menu as MenuIcon, Delete as DeleteIcon } from "@mui/icons-material";

import { RootState, AppDispatch } from "../../redux/store";
import { deleteBoard } from "../../store/slices/kanbanSlice";

const AppBar = styled(MuiAppBar)`
  background: ${(props) => props.theme.header.background};
  color: ${(props) => props.theme.header.color};
`;

const IconButton = styled(MuiIconButton)`
  svg {
    width: 22px;
    height: 22px;
  }
`;

type NavbarProps = {
  onDrawerToggle: React.MouseEventHandler<HTMLElement>;
};

const Navbar: React.FC<NavbarProps> = ({ onDrawerToggle }) => {
  const dispatch = useDispatch<AppDispatch>();
  const { boards, activeBoardId } = useSelector((state: RootState) => state.kanban);
  const activeBoard = boards.find((b) => b.id === activeBoardId);
  const boardName = activeBoard?.name || "Kanban Board";

  const handleDelete = () => {
    if (!activeBoardId) return;
    if (window.confirm(`Delete board "${boardName}"? All cards in this board will be lost.`)) {
      dispatch(deleteBoard(activeBoardId));
    }
  };

  return (
    <React.Fragment>
      <AppBar position="sticky" elevation={0}>
        <Toolbar>
          <Grid container alignItems="center">
            <Grid item sx={{ display: { xs: "block", md: "none" } }}>
              <IconButton
                color="inherit"
                aria-label="Open drawer"
                onClick={onDrawerToggle}
                size="large"
              >
                <MenuIcon />
              </IconButton>
            </Grid>
            <Grid item>
              <Typography sx={{ fontWeight: 600, color: '#000', fontSize: '1.3rem' }}>
                {boardName}
              </Typography>
            </Grid>
            <Grid item xs />
            {activeBoardId && (
              <Grid item>
                <IconButton
                  size="small"
                  onClick={handleDelete}
                  sx={{ color: '#d32f2f' }}
                  aria-label="Delete board"
                >
                  <DeleteIcon sx={{ fontSize: '18px' }} />
                </IconButton>
              </Grid>
            )}
          </Grid>
        </Toolbar>
      </AppBar>
    </React.Fragment>
  );
};

export default withTheme(Navbar);
