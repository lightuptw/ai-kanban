import React, { useState, useEffect } from "react";
import styled from "@emotion/styled";
import { withTheme } from "@emotion/react";
import { useSelector, useDispatch } from "react-redux";

import {
  Grid,
  AppBar as MuiAppBar,
  IconButton as MuiIconButton,
  Toolbar,
  Typography,
  InputBase,
  ClickAwayListener,
} from "@mui/material";


import { Menu as MenuIcon, Edit as EditIcon, Check as CheckIcon } from "@mui/icons-material";

import { RootState, AppDispatch } from "../../redux/store";
import { updateBoard } from "../../store/slices/kanbanSlice";

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

const BoardTitleInput = styled(InputBase)`
  font-size: 1.25rem;
  font-weight: 600;
  color: inherit;
  & .MuiInputBase-input {
    padding: 2px 8px;
    border: 1px solid ${(props) => props.theme.palette.divider};
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
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

  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState(boardName);

  useEffect(() => {
    setEditName(boardName);
  }, [boardName]);

  const handleSave = () => {
    const trimmed = editName.trim();
    if (trimmed && trimmed !== boardName && activeBoardId) {
      dispatch(updateBoard({ id: activeBoardId, data: { name: trimmed } }));
    } else {
      setEditName(boardName);
    }
    setEditing(false);
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
            <Grid item sx={{ display: "flex", alignItems: "center", gap: 1 }}>
              {editing ? (
                <ClickAwayListener onClickAway={handleSave}>
                  <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                    <BoardTitleInput
                      value={editName}
                      onChange={(e) => setEditName(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleSave();
                        if (e.key === "Escape") { setEditName(boardName); setEditing(false); }
                      }}
                      autoFocus
                    />
                    <IconButton color="inherit" size="small" onClick={handleSave}>
                      <CheckIcon fontSize="small" />
                    </IconButton>
                  </div>
                </ClickAwayListener>
              ) : (
                <>
                  <Typography sx={{ fontWeight: 600, color: '#000', fontSize: '1.3rem' }}>
                    {boardName}
                  </Typography>
                  <IconButton size="small" onClick={() => setEditing(true)} sx={{ color: '#000', p: 0.5 }}>
                    <EditIcon sx={{ fontSize: '14px' }} />
                  </IconButton>
                </>
              )}
            </Grid>
            <Grid item xs />
          </Grid>
        </Toolbar>
      </AppBar>
    </React.Fragment>
  );
};

export default withTheme(Navbar);
