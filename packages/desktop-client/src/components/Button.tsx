import { cn } from "cnfast";
import type { ButtonHTMLAttributes, ReactNode } from "react";

type ButtonProps = {
  children: ReactNode;
  variant?: "solid" | "outline";
} & ButtonHTMLAttributes<HTMLButtonElement>;

const baseClasses =
  "relative inline-flex items-center justify-center rounded-lg border px-3 py-1.5 text-sm/6 font-semibold focus:outline-2 focus:outline-offset-2 focus:outline-blue-500 disabled:opacity-50";

const variantClasses = {
  solid:
    "border-white/5 bg-zinc-700 text-white shadow-sm hover:bg-zinc-600 disabled:hover:bg-zinc-700",
  outline:
    "border-white/15 bg-transparent text-zinc-100 hover:bg-white/5 disabled:hover:bg-transparent",
};

export function Button({
  children,
  className = "",
  variant = "solid",
  ...props
}: ButtonProps): ReactNode {
  return (
    <button
      type="button"
      {...props}
      className={cn(baseClasses, variantClasses[variant], className)}
    >
      {children}
    </button>
  );
}
