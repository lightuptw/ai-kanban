import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  Accordion,
  AccordionDetails,
  AccordionSummary,
  Alert,
  Avatar,
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
  Typography,
} from "@mui/material";
import { CameraAlt, ExpandMore } from "@mui/icons-material";
import { useAuth } from "../hooks/useAuth";
import { api } from "../services/api";
import { authService } from "../services/auth";
import { API_BASE_URL } from "../constants";

interface UserProfileDialogProps {
  open: boolean;
  onClose: () => void;
  onboardingMode?: boolean;
}

type ProfileFormState = {
  nickname: string;
  first_name: string;
  last_name: string;
  email: string;
};

type PasswordFormState = {
  current_password: string;
  new_password: string;
  confirm_new_password: string;
};

const EMPTY_PROFILE_FORM: ProfileFormState = {
  nickname: "",
  first_name: "",
  last_name: "",
  email: "",
};

const EMPTY_PASSWORD_FORM: PasswordFormState = {
  current_password: "",
  new_password: "",
  confirm_new_password: "",
};

const UserProfileDialog: React.FC<UserProfileDialogProps> = ({ open, onClose, onboardingMode = false }) => {
  const { user, updateUser } = useAuth();
  const [form, setForm] = useState<ProfileFormState>(EMPTY_PROFILE_FORM);
  const [passwordForm, setPasswordForm] = useState<PasswordFormState>(EMPTY_PASSWORD_FORM);
  const [pendingAvatarFile, setPendingAvatarFile] = useState<File | null>(null);
  const [avatarPreviewUrl, setAvatarPreviewUrl] = useState<string | null>(null);
  const [saveLoading, setSaveLoading] = useState(false);
  const [passwordLoading, setPasswordLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [passwordSuccess, setPasswordSuccess] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const setField = (field: keyof ProfileFormState) => (event: React.ChangeEvent<HTMLInputElement>) =>
    setForm((prev) => ({ ...prev, [field]: event.target.value }));

  const setPasswordField =
    (field: keyof PasswordFormState) => (event: React.ChangeEvent<HTMLInputElement>) =>
      setPasswordForm((prev) => ({ ...prev, [field]: event.target.value }));

  useEffect(() => {
    if (!open) {
      return;
    }

    if (avatarPreviewUrl) {
      URL.revokeObjectURL(avatarPreviewUrl);
    }

    setForm({
      nickname: user?.nickname || "",
      first_name: user?.first_name || "",
      last_name: user?.last_name || "",
      email: user?.email || "",
    });
    setPasswordForm(EMPTY_PASSWORD_FORM);
    setPendingAvatarFile(null);
    setAvatarPreviewUrl(null);
    setError(null);
    setSuccess(null);
    setPasswordError(null);
    setPasswordSuccess(null);
  }, [avatarPreviewUrl, open, user]);

  useEffect(() => {
    return () => {
      if (avatarPreviewUrl) {
        URL.revokeObjectURL(avatarPreviewUrl);
      }
    };
  }, [avatarPreviewUrl]);

  const initials = useMemo(() => {
    const fallback = form.nickname || user?.nickname || user?.username || "User";
    return fallback.trim().charAt(0).toUpperCase() || "U";
  }, [form.nickname, user]);

  const existingAvatarUrl = user?.avatar_url ? `${API_BASE_URL}${user.avatar_url}` : undefined;
  const avatarSrc = avatarPreviewUrl || existingAvatarUrl;
  const nicknameValid = form.nickname.trim().length > 0;
  const canClose = !onboardingMode || nicknameValid;

  const handleDialogClose = () => {
    if (!canClose || saveLoading) {
      return;
    }
    onClose();
  };

  const handleAvatarClick = () => {
    fileInputRef.current?.click();
  };

  const handleAvatarSelected = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) {
      return;
    }

    if (avatarPreviewUrl) {
      URL.revokeObjectURL(avatarPreviewUrl);
    }

    setPendingAvatarFile(file);
    setAvatarPreviewUrl(URL.createObjectURL(file));
  };

  const handleSave = async () => {
    if (!nicknameValid) {
      setError("Nickname is required.");
      return;
    }

    setSaveLoading(true);
    setError(null);
    setSuccess(null);

    try {
      if (pendingAvatarFile) {
        await api.uploadAvatar(pendingAvatarFile);
      }

      await api.updateProfile({
        nickname: form.nickname.trim(),
        first_name: form.first_name.trim(),
        last_name: form.last_name.trim(),
        email: form.email.trim(),
      });

      const freshUser = await api.getMe();
      updateUser(freshUser);

      if (avatarPreviewUrl) {
        URL.revokeObjectURL(avatarPreviewUrl);
      }
      setAvatarPreviewUrl(null);
      setPendingAvatarFile(null);
      setSuccess("Profile updated successfully.");
      onClose();
    } catch (saveError) {
      const message = saveError instanceof Error ? saveError.message : "Failed to update profile.";
      setError(message);
    } finally {
      setSaveLoading(false);
    }
  };

  const handleChangePassword = async () => {
    setPasswordError(null);
    setPasswordSuccess(null);

    if (!passwordForm.current_password) {
      setPasswordError("Current password is required.");
      return;
    }
    if (passwordForm.new_password.length < 8) {
      setPasswordError("New password must be at least 8 characters.");
      return;
    }
    if (passwordForm.new_password !== passwordForm.confirm_new_password) {
      setPasswordError("New password and confirmation do not match.");
      return;
    }

    setPasswordLoading(true);
    try {
      const response = await api.changePassword({
        current_password: passwordForm.current_password,
        new_password: passwordForm.new_password,
      });
      setPasswordSuccess(response.message);
      setPasswordForm(EMPTY_PASSWORD_FORM);
    } catch (changeError) {
      const message = changeError instanceof Error ? changeError.message : "Failed to change password.";
      setPasswordError(message);
    } finally {
      setPasswordLoading(false);
    }
  };

  return (
    <Dialog
      open={open}
      onClose={handleDialogClose}
      maxWidth="sm"
      fullWidth
      disableEscapeKeyDown={onboardingMode && !canClose}
    >
      <DialogTitle>{onboardingMode ? "Welcome! Set up your profile" : "Edit Profile"}</DialogTitle>
      <DialogContent dividers>
        {onboardingMode ? (
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            Tell us a little about you so your workspace feels personal from day one.
          </Typography>
        ) : null}

        {error ? (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        ) : null}

        {success ? (
          <Alert severity="success" sx={{ mb: 2 }}>
            {success}
          </Alert>
        ) : null}

        <Box sx={{ display: "flex", justifyContent: "center", mb: 3 }}>
          <Box
            sx={{
              width: 80,
              height: 80,
              borderRadius: "50%",
              position: "relative",
              cursor: "pointer",
              overflow: "hidden",
            }}
            onClick={handleAvatarClick}
          >
            <Avatar src={avatarSrc} sx={{ width: 80, height: 80 }}>
              {initials}
            </Avatar>
            <Box
              sx={{
                position: "absolute",
                inset: 0,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                bgcolor: "rgba(0, 0, 0, 0.45)",
                opacity: 0,
                transition: "opacity 180ms ease",
                "&:hover": {
                  opacity: 1,
                },
              }}
            >
              <CameraAlt sx={{ color: "#fff" }} />
            </Box>
          </Box>
          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            hidden
            onChange={handleAvatarSelected}
          />
        </Box>

        <Box sx={{ display: "grid", gap: 2 }}>
          <TextField
            label="Nickname"
            value={form.nickname}
            onChange={setField("nickname")}
            fullWidth
            required
          />
          <TextField label="First Name" value={form.first_name} onChange={setField("first_name")} fullWidth />
          <TextField label="Last Name" value={form.last_name} onChange={setField("last_name")} fullWidth />
          <TextField label="Email" type="email" value={form.email} onChange={setField("email")} fullWidth />
        </Box>

        <Accordion sx={{ mt: 3 }}>
          <AccordionSummary expandIcon={<ExpandMore />}>
            <Typography variant="subtitle1">Change Password</Typography>
          </AccordionSummary>
          <AccordionDetails>
            {passwordError ? (
              <Alert severity="error" sx={{ mb: 2 }}>
                {passwordError}
              </Alert>
            ) : null}
            {passwordSuccess ? (
              <Alert severity="success" sx={{ mb: 2 }}>
                {passwordSuccess}
              </Alert>
            ) : null}
            <Box sx={{ display: "grid", gap: 2 }}>
              <TextField
                label="Current Password"
                type="password"
                value={passwordForm.current_password}
                onChange={setPasswordField("current_password")}
                fullWidth
              />
              <TextField
                label="New Password"
                type="password"
                value={passwordForm.new_password}
                onChange={setPasswordField("new_password")}
                helperText="Minimum 8 characters"
                fullWidth
              />
              <TextField
                label="Confirm New Password"
                type="password"
                value={passwordForm.confirm_new_password}
                onChange={setPasswordField("confirm_new_password")}
                fullWidth
              />
              <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
                <Button variant="outlined" onClick={handleChangePassword} disabled={passwordLoading}>
                  {passwordLoading ? <CircularProgress size={18} color="inherit" /> : "Update Password"}
                </Button>
              </Box>
            </Box>
          </AccordionDetails>
        </Accordion>

        <Box sx={{ mt: 3, display: "flex", justifyContent: "flex-start" }}>
          <Button color="error" onClick={authService.logout}>
            Logout
          </Button>
        </Box>
      </DialogContent>
      <DialogActions>
        <Button onClick={handleDialogClose} disabled={!canClose || saveLoading}>
          Cancel
        </Button>
        <Button variant="contained" onClick={handleSave} disabled={saveLoading || !nicknameValid}>
          {saveLoading ? <CircularProgress size={18} color="inherit" /> : "Save"}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

export default UserProfileDialog;
