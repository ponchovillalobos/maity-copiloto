import React from "react";
import Image from "next/image";
import { Dialog, DialogContent, DialogTitle, DialogTrigger } from "./ui/dialog";
import { VisuallyHidden } from "./ui/visually-hidden";
import { About } from "./About";

interface LogoProps {
    isCollapsed: boolean;
}

const Logo = React.forwardRef<HTMLButtonElement, LogoProps>(({ isCollapsed }, ref) => {
  return (
    <Dialog>
      {isCollapsed ? (
        <DialogTrigger asChild>
          <button ref={ref} className="flex items-center justify-start mb-2 cursor-pointer bg-transparent border-none p-0 hover:opacity-80 transition-opacity">
            <Image src="/logo-collapsed.png" alt="Maity Logo" width={40} height={40} />
          </button>
        </DialogTrigger>
      ) : (
        <DialogTrigger asChild>
          <span className="text-lg text-center border rounded-full bg-[#f0f2fe] dark:bg-blue-900/30 border-white font-semibold text-gray-100 dark:text-gray-200 mb-2 block items-center cursor-pointer hover:opacity-80 transition-opacity">
            <span>Maity</span>
          </span>
        </DialogTrigger>
      )}
      <DialogContent>
        <VisuallyHidden>
          <DialogTitle>Acerca de Maity</DialogTitle>
        </VisuallyHidden>
        <About />
      </DialogContent>
    </Dialog>
  );
});

Logo.displayName = "Logo";

export default Logo;