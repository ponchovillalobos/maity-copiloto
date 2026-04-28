import React, { useEffect } from 'react';
import { useOnboarding } from '@/contexts/OnboardingContext';
import {
  WelcomeStep,
  PermissionsStep,
  DownloadProgressStep,
  SetupOverviewStep,
  AutoModelSetupStep,
  MicTestStep,
} from './steps';

interface OnboardingFlowProps {
  onComplete: () => void;
}

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const { currentStep } = useOnboarding();
  const [isMac, setIsMac] = React.useState(false);

  useEffect(() => {
    // Check if running on macOS
    const checkPlatform = async () => {
      try {
        // Dynamic import to avoid SSR issues if any
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch (e) {
        console.error('Failed to detect platform:', e);
        // Fallback
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };
    checkPlatform();
  }, []);

  // 6-Step Onboarding Flow (System-Recommended Models):
  // Step 1: Welcome - Introduce Maity features
  // Step 2: Setup Overview - Database initialization + show recommended downloads
  // Step 3: Auto Model Setup - Download Ollama models (gemma3:4b, nomic-embed-text)
  // Step 4: Download Progress - Download Parakeet (auto-selected based on RAM)
  // Step 5: Mic Test - Audio device selection + level monitoring + preview
  // Step 6: Permissions - Request mic + system audio (macOS only)

  return (
    <div className="onboarding-flow">
      {currentStep === 1 && <WelcomeStep />}
      {currentStep === 2 && <SetupOverviewStep />}
      {currentStep === 3 && <AutoModelSetupStep />}
      {currentStep === 4 && <DownloadProgressStep />}
      {currentStep === 5 && <MicTestStep />}
      {currentStep === 6 && isMac && <PermissionsStep />}
    </div>
  );
}
