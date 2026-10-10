import { forwardRef, type ComponentPropsWithoutRef } from "react";

/** Nexvyn LaptopMockupCard; original structure, CSS adapted to Astrid's scale.
 * Source: https://ui.nexvyn.dev/r/laptop-mockup.json */
export const LaptopMockupCard = forwardRef<
  HTMLDivElement,
  ComponentPropsWithoutRef<"div"> & { variant?: "gray" | "titanium" }
>(({ className = "", children, variant = "gray", ...props }, ref) => (
  <div
    ref={ref}
    data-slot="laptop-mockup-card"
    data-variant={variant}
    className={`laptop-mockup ${className}`}
    {...props}
  >
    <div className="laptop-frame">
      <div className="laptop-bezel">
        <div className="laptop-screen">
          <div className="laptop-content">{children}</div>
        </div>
      </div>
    </div>
    <div className="laptop-base">
      <div className="laptop-notch" aria-hidden="true" />
    </div>
  </div>
));
LaptopMockupCard.displayName = "LaptopMockupCard";
