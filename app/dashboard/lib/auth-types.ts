export type User = {
  id: string;
  name: string;
  email: string;
  avatar: string | null;
  emailVerified: boolean;
  createdAt: string;
  lastLoginAt: string | null;
};

export type Session = {
  user: User;
  token: string;
  expiresAt: string;
  /** True when the account already owns an org + project (skips onboarding). */
  onboardingCompleted: boolean;
};

export type OAuthProvider = "github" | "google";

export type AuthError = {
  code: string;
  message: string;
};

export type PasswordRequirements = {
  minLength: boolean;
  hasUppercase: boolean;
  hasLowercase: boolean;
  hasNumber: boolean;
  hasSpecialChar: boolean;
};

export const PASSWORD_RULES = {
  minLength: 8,
  patterns: {
    uppercase: /[A-Z]/,
    lowercase: /[a-z]/,
    number: /[0-9]/,
    specialChar: /[!@#$%^&*()\-_=+\[\]{};':"\\|,.<>\/?`~]/,
  },
} as const;

export function validatePassword(password: string): PasswordRequirements {
  return {
    minLength: password.length >= PASSWORD_RULES.minLength,
    hasUppercase: PASSWORD_RULES.patterns.uppercase.test(password),
    hasLowercase: PASSWORD_RULES.patterns.lowercase.test(password),
    hasNumber: PASSWORD_RULES.patterns.number.test(password),
    hasSpecialChar: PASSWORD_RULES.patterns.specialChar.test(password),
  };
}

export function getPasswordStrength(requirements: PasswordRequirements): {
  score: number;
  label: string;
  color: string;
} {
  const score = Object.values(requirements).filter(Boolean).length;

  if (score <= 1) return { score, label: "Weak", color: "bg-destructive" };
  if (score === 2) return { score, label: "Fair", color: "bg-orange-500" };
  if (score === 3) return { score, label: "Good", color: "bg-yellow-500" };
  if (score === 4) return { score, label: "Strong", color: "bg-success" };
  return { score, label: "Very Strong", color: "bg-emerald-400" };
}
