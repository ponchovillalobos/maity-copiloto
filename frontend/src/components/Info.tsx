import React from "react";
import { Info as InfoIcon } from "lucide-react";
import { Dialog, DialogContent, DialogTitle, DialogTrigger } from "./ui/dialog";
import { VisuallyHidden } from "./ui/visually-hidden";
import { About } from "./About";

interface InfoProps {
    isCollapsed: boolean;
}

const Info = React.forwardRef<HTMLButtonElement, InfoProps>(({ isCollapsed }, ref) => {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <button 
          ref={ref} 
          className={`flex items-center justify-center mb-2 cursor-pointer border-none transition-colors ${
            isCollapsed 
              ? "bg-transparent p-2 hover:bg-[#e7e7e9] dark:hover:bg-gray-700 rounded-lg"
              : "w-full px-3 py-1.5 mt-1 text-sm font-medium text-gray-100 dark:text-gray-200 bg-[#d0d0d3] dark:bg-gray-600 hover:bg-[#d0d0d3] dark:hover:bg-gray-600 rounded-lg shadow-sm"
          }`}
          title="Acerca de Maity"
        >
          <InfoIcon className={`text-gray-200 dark:text-gray-300 ${isCollapsed ? "w-5 h-5" : "w-4 h-4"}`} />
          {!isCollapsed && (
            <span className="ml-2 text-sm text-gray-100 dark:text-gray-200">About</span>
          )}
        </button>
      </DialogTrigger>
      <DialogContent>
        <VisuallyHidden>
          <DialogTitle>Acerca de Maity</DialogTitle>
        </VisuallyHidden>
        <About />
      </DialogContent>
    </Dialog>
  );
});

Info.displayName = "About";

export default Info; 