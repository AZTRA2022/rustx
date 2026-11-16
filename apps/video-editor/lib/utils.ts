import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/** What every shadcn component uses to merge its classes. */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
