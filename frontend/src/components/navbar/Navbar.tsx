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
  FormControl,
  Select,
  MenuItem,
} from "@mui/material";


import { Menu as MenuIcon, Edit as EditIcon, Check as CheckIcon, Delete as DeleteIcon } from "@mui/icons-material";

import { RootState, AppDispatch } from "../../redux/store";
import { updateBoard, deleteBoard } from "../../store/slices/kanbanSlice";
import { api } from "../../services/api";

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
  const [aiConcurrency, setAiConcurrency] = useState("1");

  useEffect(() => {
    setEditName(boardName);
  }, [boardName]);

  useEffect(() => {
    api
      .getSetting("ai_concurrency")
      .then((setting) => {
        const parsed = Number.parseInt(setting.value, 10);
        setAiConcurrency(Number.isFinite(parsed) && parsed >= 1 && parsed <= 5 ? String(parsed) : "1");
      })
      .catch(() => {
        setAiConcurrency("1");
      });
  }, []);

  const handleConcurrencyChange = (value: string) => {
    setAiConcurrency(value);
    api.setSetting("ai_concurrency", value).catch(() => {
      setAiConcurrency("1");
    });
  };

  const handleSave = () => {
    const trimmed = editName.trim();
    if (trimmed && trimmed !== boardName && activeBoardId) {
      dispatch(updateBoard({ id: activeBoardId, data: { name: trimmed } }));
    } else {
      setEditName(boardName);
    }
    setEditing(false);
  };

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
            <Grid item sx={{ display: "flex", alignItems: "center", mr: 1.5, gap: 1 }}>
              <Typography sx={{ color: "#000", fontSize: "0.8rem", fontWeight: 600 }}>
                AI
              </Typography>
              <FormControl size="small" sx={{ minWidth: 68 }}>
                <Select
                  value={aiConcurrency}
                  onChange={(e) => handleConcurrencyChange(e.target.value as string)}
                  sx={{
                    height: 28,
                    color: "#000",
                    fontSize: "0.8rem",
                    "& .MuiOutlinedInput-notchedOutline": {
                      borderColor: "rgba(0,0,0,0.25)",
                    },
                    "& .MuiSelect-select": {
                      py: 0.25,
                      pr: 3,
                    },
                  }}
                >
                  {[1, 2, 3, 4, 5].map((value) => (
                    <MenuItem key={value} value={String(value)}>
                      {value}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
              <Typography sx={{ color: "#000", fontSize: "0.75rem" }}>
                concurrent
              </Typography>
            </Grid>
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
