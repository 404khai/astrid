import { forwardRef, type ComponentPropsWithoutRef } from "react";

/** Nexvyn PhoneMockupCard; original structure (chassis, side controls,
 * screen, dynamic island, home indicator) with scoped CSS instead of Tailwind.
 * Source: https://ui.nexvyn.dev/r/phone-mockup.json */
export const PhoneMockupCard = forwardRef<
  HTMLDivElement,
  ComponentPropsWithoutRef<"div"> & {
    variant?: "gray" | "titanium";
    showDynamicIsland?: boolean;
  }
>(
  (
    { className = "", children, variant = "titanium", showDynamicIsland = true, ...props },
    ref,
  ) => (
    <div
      ref={ref}
      data-slot="phone-mockup-card"
      data-variant={variant}
      className={`phone-mockup ${className}`}
      {...props}
    >
      <i className="phone-control" data-at="action" aria-hidden="true" />
      <i className="phone-control" data-at="up" aria-hidden="true" />
      <i className="phone-control" data-at="down" aria-hidden="true" />
      <i className="phone-control" data-at="power" aria-hidden="true" />
      <div className="phone-screen">
        <div className="phone-display">{children}</div>
        {showDynamicIsland && (
          <div className="phone-island" aria-hidden="true">
            <i />
          </div>
        )}
        <div className="phone-home" aria-hidden="true" />
      </div>
    </div>
  ),
);
PhoneMockupCard.displayName = "PhoneMockupCard";
