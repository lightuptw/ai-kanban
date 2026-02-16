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

const LoginPage: React.FC = () => {
  const navigate = useNavigate();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setLoading(true);
    setError(null);

    try {
      await authService.login(username.trim(), password);
      window.dispatchEvent(new Event("auth:login"));
      navigate("/");
    } catch (submitError) {
      const message = submitError instanceof Error ? submitError.message : "Login failed";
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
          Welcome back
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
          Sign in to continue to your board.
        </Typography>

        {error ? (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        ) : null}

        <Box component="form" onSubmit={handleSubmit}>
          <TextField
            label="Username"
            value={username}
            onChange={(event) => setUsername(event.target.value)}
            fullWidth
            margin="normal"
            required
            autoFocus
          />
          <TextField
            label="Password"
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            fullWidth
            margin="normal"
            required
          />

          <Button type="submit" variant="contained" fullWidth disabled={loading} sx={{ mt: 2, mb: 2 }}>
            {loading ? <CircularProgress size={22} color="inherit" /> : "Login"}
          </Button>
        </Box>

        <Typography variant="body2" color="text.secondary" textAlign="center">
          Don&apos;t have an account?{" "}
          <Link component={RouterLink} to="/register" underline="hover">
            Register
          </Link>
        </Typography>
      </Paper>
    </Box>
  );
};

export default LoginPage;
