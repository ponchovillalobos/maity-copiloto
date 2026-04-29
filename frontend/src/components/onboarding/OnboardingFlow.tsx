import React, { useEffect } from 'react';
import { useOnboarding } from '@/contexts/OnboardingContext';
import {
  WelcomeStep,
  PermissionsStep,
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
    const checkPlatform = async () => {
      try {
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch (e) {
        console.error('Failed to detect platform:', e);
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };
    checkPlatform();
  }, []);

  // Flow ULTRA simple para usuarios no-técnicos. Cero opciones, cero clicks
  // técnicos. Sólo 3 pantallas (4 en macOS):
  //   1. Bienvenida ("Comenzar")
  //   2. Descarga AUTO de los 2 modelos (Gemma + Parakeet, en paralelo,
  //      avanza solo al terminar)
  //   3. Prueba de micrófono
  //   4. Permisos (solo macOS)
  //
  // El Setup Overview y el Download Progress separados ya no existen — todo
  // se consolida en AutoModelSetupStep.

  return (
    <div className="onboarding-flow">
      {currentStep === 1 && <WelcomeStep />}
      {currentStep === 2 && <AutoModelSetupStep />}
      {currentStep === 3 && <MicTestStep />}
      {currentStep === 4 && isMac && <PermissionsStep />}
    </div>
  );
}
