import React, { useState } from "react";
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Link,
  Paper,
  TextField,
  Typography,
} from "@mui/material";
import { Link as RouterLink, useNavigate } from "react-router-dom";
import { authService } from "../../services/auth";

type RegisterFormState = {
  username: string;
  nickname: string;
  password: string;
  confirmPassword: string;
  first_name: string;
  last_name: string;
  email: string;
};

const RegisterPage: React.FC = () => {
  const navigate = useNavigate();
  const [form, setForm] = useState<RegisterFormState>({
    username: "",
    nickname: "",
    password: "",
    confirmPassword: "",
    first_name: "",
    last_name: "",
    email: "",
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const setField = (field: keyof RegisterFormState) => (event: React.ChangeEvent<HTMLInputElement>) => {
    setForm((prev) => ({ ...prev, [field]: event.target.value }));
  };

  const validate = () => {
    if (form.username.trim().length < 3) {
      return "Username must be at least 3 characters.";
    }
    if (!form.nickname.trim()) {
      return "Nickname is required.";
    }
    if (form.password.length < 8) {
      return "Password must be at least 8 characters.";
    }
    if (form.password !== form.confirmPassword) {
      return "Passwords do not match.";
    }
    return null;
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);

    const validationError = validate();
    if (validationError) {
      setError(validationError);
      return;
    }

    setLoading(true);
    try {
      await authService.register({
        username: form.username.trim(),
        nickname: form.nickname.trim(),
        password: form.password,
        first_name: form.first_name.trim() || undefined,
        last_name: form.last_name.trim() || undefined,
        email: form.email.trim() || undefined,
      });
      navigate("/");
    } catch (submitError) {
      const message = submitError instanceof Error ? submitError.message : "Registration failed";
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Box
      sx={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        bgcolor: "background.default",
        p: 2,
      }}
    >
      <Paper elevation={4} sx={{ width: "100%", maxWidth: 440, p: 4 }}>
        <Typography variant="h4" component="h1" gutterBottom>
          Create account
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
          Set up your profile to start managing work.
        </Typography>

        {error ? (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        ) : null}

        <Box component="form" onSubmit={handleSubmit}>
          <TextField
            label="Username"
            value={form.username}
            onChange={setField("username")}
            fullWidth
            margin="normal"
            required
            helperText="Min 3 characters"
            autoFocus
          />
          <TextField
            label="Nickname"
            value={form.nickname}
            onChange={setField("nickname")}
            fullWidth
            margin="normal"
            required
            helperText="How AI will address you"
          />
          <TextField
            label="Password"
            type="password"
            value={form.password}
            onChange={setField("password")}
            fullWidth
            margin="normal"
            required
            helperText="Min 8 characters"
          />
          <TextField
            label="Confirm Password"
            type="password"
            value={form.confirmPassword}
            onChange={setField("confirmPassword")}
            fullWidth
            margin="normal"
            required
          />
          <TextField
            label="First Name"
            value={form.first_name}
            onChange={setField("first_name")}
            fullWidth
            margin="normal"
          />
          <TextField
            label="Last Name"
            value={form.last_name}
            onChange={setField("last_name")}
            fullWidth
            margin="normal"
          />
          <TextField
            label="Email"
            type="email"
            value={form.email}
            onChange={setField("email")}
            fullWidth
            margin="normal"
          />

          <Button type="submit" variant="contained" fullWidth disabled={loading} sx={{ mt: 2, mb: 2 }}>
            {loading ? <CircularProgress size={22} color="inherit" /> : "Register"}
          </Button>
        </Box>

        <Typography variant="body2" color="text.secondary" textAlign="center">
          Already have an account?{" "}
          <Link component={RouterLink} to="/login" underline="hover">
            Login
          </Link>
        </Typography>
      </Paper>
    </Box>
  );
};

export default RegisterPage;
