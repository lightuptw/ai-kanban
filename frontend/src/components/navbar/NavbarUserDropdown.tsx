import React, { useState, useCallback } from "react";
import {
  Tooltip,
  Menu,
  MenuItem,
  Divider,
  IconButton,
} from "@mui/material";
import UserAvatar from "../UserAvatar";
import AvatarUploadDialog from "../AvatarUploadDialog";
import { getUser, setUser, logout } from "../services/auth";
import type { AuthUser } from "../services/auth";

function NavbarUserDropdown() {
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
  const [avatarDialogOpen, setAvatarDialogOpen] = useState(false);
  const [user, setLocalUser] = useState<AuthUser | null>(getUser);

  const handleOpen = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget);
  };

  const handleClose = () => {
    setAnchorEl(null);
  };

  const handleAvatarSuccess = useCallback((updatedUser: AuthUser) => {
    setUser(updatedUser);
    setLocalUser(updatedUser);
  }, []);

  return (
    <React.Fragment>
      <Tooltip title={user?.nickname || "Account"}>
        <IconButton color="inherit" size="large" onClick={handleOpen}>
          <UserAvatar
            userId={user?.id}
            nickname={user?.nickname || "?"}
            hasAvatar={user?.has_avatar}
            size={28}
          />
        </IconButton>
      </Tooltip>
      <Menu
        anchorEl={anchorEl}
        open={Boolean(anchorEl)}
        onClose={handleClose}
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        transformOrigin={{ vertical: "top", horizontal: "right" }}
      >
        <MenuItem
          onClick={() => {
            handleClose();
            setAvatarDialogOpen(true);
          }}
        >
          Change Avatar
        </MenuItem>
        <Divider />
        <MenuItem onClick={logout}>Sign Out</MenuItem>
      </Menu>
      <AvatarUploadDialog
        open={avatarDialogOpen}
        onClose={() => setAvatarDialogOpen(false)}
        onSuccess={handleAvatarSuccess}
      />
    </React.Fragment>
  );
}

export default NavbarUserDropdown;
