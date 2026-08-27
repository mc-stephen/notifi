import { create } from "zustand";

const STEP_ROUTES = [
  "/onboarding/welcome",
  "/onboarding/use-case",
  "/onboarding/organization",
  "/onboarding/project",
  "/onboarding/setup-channels",
  "/onboarding/invite-team",
  "/onboarding/success",
] as const;

type OnboardingData = {
  currentStep: number;
  isCompleted: boolean;
  useCase: string | null;
  orgName: string;
  orgLogo: string | null;
  projectName: string;
  projectDescription: string;
  selectedChannels: string[];
  teamEmails: string[];
};

type OnboardingStore = OnboardingData & {
  setStep: (step: number) => void;
  nextStep: () => void;
  prevStep: () => void;
  completeOnboarding: () => void;
  updateData: (data: Partial<OnboardingData>) => void;
  getStepRoute: (step: number) => string;
  getTotalSteps: () => number;
};

const initialState: OnboardingData = {
  currentStep: 0,
  isCompleted: false,
  useCase: null,
  orgName: "",
  orgLogo: null,
  projectName: "",
  projectDescription: "",
  selectedChannels: [],
  teamEmails: [],
};

export const useOnboardingStore = create<OnboardingStore>((set, get) => ({
  ...initialState,

  setStep: (step) => set({ currentStep: step }),

  nextStep: () => {
    const { currentStep } = get();
    if (currentStep < STEP_ROUTES.length - 1) {
      set({ currentStep: currentStep + 1 });
    }
  },

  prevStep: () => {
    const { currentStep } = get();
    if (currentStep > 0) {
      set({ currentStep: currentStep - 1 });
    }
  },

  completeOnboarding: () => set({ isCompleted: true }),

  updateData: (data) => set(data),

  getStepRoute: (step) => STEP_ROUTES[step] || STEP_ROUTES[0],

  getTotalSteps: () => STEP_ROUTES.length,
}));

export { STEP_ROUTES };
