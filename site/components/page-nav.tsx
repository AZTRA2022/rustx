import { cn } from "@/lib/utils"

export function PageNav({
  children,
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div className={cn(" scroll-mt-24", className)} {...props}>
      <div className=" flex items-center justify-between gap-4 py-4">
        {children}
      </div>
    </div>
  )
}
