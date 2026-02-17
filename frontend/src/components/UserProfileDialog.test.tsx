import type { ReactElement } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ThemeProvider, createTheme } from "@mui/material/styles";
import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";
import type { AuthUser } from "../services/auth";
import UserProfileDialog from "./UserProfileDialog";
import { useAuth } from "../hooks/useAuth";
import { api } from "../services/api";
import { authService } from "../services/auth";

vi.mock("../hooks/useAuth", () => ({
  useAuth: vi.fn(() => ({
    user: {
      id: "u1",
      username: "test",
      nickname: "TestUser",
      first_name: "",
      last_name: "",
      email: "test@test.com",
      tenant_id: "t1",
      avatar_url: null,
      profile_completed: true,
    },
    updateUser: vi.fn(),
    refreshUser: vi.fn(),
  })),
}));

vi.mock("../services/api", () => ({
  api: {
    getMe: vi.fn(),
    updateProfile: vi.fn(),
    uploadAvatar: vi.fn(),
    changePassword: vi.fn(),
    getAvatarUrl: vi.fn((id: string) => `/api/users/${id}/avatar`),
  },
}));

vi.mock("../services/auth", () => ({
  authService: {
    logout: vi.fn(),
  },
}));

vi.mock("../constants", () => ({ API_BASE_URL: "" }));

const baseUser: AuthUser = {
  id: "u1",
  username: "test",
  nickname: "TestUser",
  first_name: "",
  last_name: "",
  email: "test@test.com",
  tenant_id: "t1",
  avatar_url: null,
  profile_completed: true,
};

const renderWithTheme = (ui: ReactElement) => {
  const theme = createTheme();
  return render(<ThemeProvider theme={theme}>{ui}</ThemeProvider>);
};

describe("UserProfileDialog", () => {
  const mockedUseAuth = vi.mocked(useAuth);
  const updateUser = vi.fn();

  beforeEach(() => {
    mockedUseAuth.mockReturnValue({
      user: baseUser,
      updateUser,
      refreshUser: vi.fn(),
    });

    vi.mocked(api.updateProfile).mockResolvedValue(baseUser);
    vi.mocked(api.getMe).mockResolvedValue(baseUser);
    vi.mocked(api.uploadAvatar).mockResolvedValue(baseUser);
    vi.mocked(api.changePassword).mockResolvedValue({ message: "Password updated" });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders with user data pre-filled", () => {
    renderWithTheme(<UserProfileDialog open onClose={vi.fn()} />);

    expect(screen.getByDisplayValue("TestUser")).toBeVisible();
    expect(screen.getByDisplayValue("test@test.com")).toBeVisible();
  });

  it("shows required error when nickname is empty", () => {
    renderWithTheme(<UserProfileDialog open onClose={vi.fn()} />);

    fireEvent.change(screen.getByLabelText(/nickname/i), { target: { value: "" } });
    const saveButton = screen.getByRole("button", { name: /save/i });
    fireEvent.click(saveButton);

    expect(saveButton).toBeDisabled();
    expect(api.updateProfile).not.toHaveBeenCalled();
  });

  it("calls api.updateProfile on save", async () => {
    const onClose = vi.fn();
    renderWithTheme(<UserProfileDialog open onClose={onClose} />);

    fireEvent.change(screen.getByLabelText(/nickname/i), { target: { value: "Updated Name" } });
    fireEvent.change(screen.getByLabelText(/first name/i), { target: { value: "Ada" } });
    fireEvent.change(screen.getByLabelText(/last name/i), { target: { value: "Lovelace" } });
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: "ada@test.com" } });
    fireEvent.click(screen.getByRole("button", { name: /save/i }));

    await waitFor(() => {
      expect(api.updateProfile).toHaveBeenCalledWith({
        nickname: "Updated Name",
        first_name: "Ada",
        last_name: "Lovelace",
        email: "ada@test.com",
      });
    });
    expect(api.getMe).toHaveBeenCalledTimes(1);
    expect(updateUser).toHaveBeenCalledWith(baseUser);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("calls api.uploadAvatar when file selected", async () => {
    const createObjectUrlSpy = vi
      .spyOn(URL, "createObjectURL")
      .mockReturnValue(null as unknown as string);

    renderWithTheme(<UserProfileDialog open onClose={vi.fn()} />);
    const fileInput = document.body.querySelector('input[type="file"]');
    const file = new File(["avatar"], "avatar.png", { type: "image/png" });

    expect(fileInput).not.toBeNull();
    fireEvent.change(fileInput as HTMLInputElement, { target: { files: [file] } });
    fireEvent.click(screen.getByRole("button", { name: /save/i }));

    await waitFor(() => {
      expect(api.uploadAvatar).toHaveBeenCalledWith(file);
    });

    createObjectUrlSpy.mockRestore();
  });

  it("onboarding mode shows welcome text", () => {
    renderWithTheme(<UserProfileDialog open onboardingMode onClose={vi.fn()} />);

    expect(screen.getByText("Welcome! Set up your profile")).toBeVisible();
    expect(
      screen.getByText("Tell us a little about you so your workspace feels personal from day one.")
    ).toBeVisible();
  });

  it("onboarding mode prevents close with empty nickname", () => {
    const onClose = vi.fn();
    mockedUseAuth.mockReturnValue({
      user: { ...baseUser, nickname: "" },
      updateUser,
      refreshUser: vi.fn(),
    });

    renderWithTheme(<UserProfileDialog open onboardingMode onClose={onClose} />);

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    fireEvent.keyDown(document, { key: "Escape", code: "Escape" });

    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByText("Welcome! Set up your profile")).toBeVisible();
  });

  it("calls authService.logout on logout click", () => {
    renderWithTheme(<UserProfileDialog open onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "Logout" }));

    expect(authService.logout).toHaveBeenCalledTimes(1);
  });
});
