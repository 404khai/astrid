import { useId } from "react";

/** The original Astrid mark, with independently animated eyes. */
export function AstridLogo({ animated = false }: { animated?: boolean }) {
  const id = useId();
  return (
    <svg
      className="astrid-logo"
      aria-hidden="true"
      viewBox="0 0 20 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M19.9531 2.89855H17.1111L14.1784 0H5.82473L2.89202 2.89855H0.0500782C0.0219092 2.89855 0 2.91735 0 2.94555V5.55895C0 5.58402 0.0219092 5.59969 0.0500782 5.59969H3.74961L6.62911 2.73561H13.4178L16.3005 5.59969H19.9531C19.9812 5.59969 20 5.58402 20 5.55895V2.94555C20 2.91735 19.9812 2.89855 19.9531 2.89855Z"
        fill={`url(#${id}-top)`}
      />
      <path
        d="M18.5666 10.3564H16.3256L13.421 13.2362H6.63854L3.74965 10.3784H0.053252C0.0219531 10.3784 0.00317383 10.3972 0.00317383 10.4222V13.0356C0.00317383 13.0607 0.0219531 13.0764 0.053252 13.0764H2.88893L5.82477 16H14.1784L17.1268 13.0764H19.9468C19.975 13.0764 19.9969 13.0607 19.9969 13.0356V10.4222C19.9969 10.3972 19.975 10.3784 19.9468 10.3784H18.5666V10.3564Z"
        fill={`url(#${id}-bottom)`}
      />
      <g className={animated ? "logo-eyes animated" : "logo-eyes"}>
        <path
          d="M8.94511 5.49634H6.04997C6.02493 5.49634 6.00928 5.51201 6.00928 5.53081V10.391C6.00928 10.4129 6.02493 10.4317 6.04997 10.4317H8.94511C8.97015 10.4317 8.98893 10.4129 8.98893 10.391V5.53081C8.98893 5.51201 8.97015 5.49634 8.94511 5.49634Z"
          fill="#00F7D5"
        />
        <path
          d="M13.9811 5.49634H11.0578C11.0359 5.49634 11.0203 5.51201 11.0203 5.53081V10.391C11.0203 10.4129 11.0359 10.4317 11.0578 10.4317H13.9811C14.0124 10.4317 14.0375 10.4098 14.0375 10.3816V5.54961C14.0375 5.51827 14.0124 5.49634 13.9811 5.49634Z"
          fill="#00F7D5"
        />
      </g>
      <defs>
        <linearGradient
          id={`${id}-top`}
          x1="3.9374"
          y1="0.908735"
          x2="17.817"
          y2="6.69738"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="#4459F9" />
          <stop offset="0.5" stopColor="#1A32EC" />
          <stop offset="1" stopColor="#4459F9" />
        </linearGradient>
        <linearGradient
          id={`${id}-bottom`}
          x1="2.05951"
          y1="9.55739"
          x2="15.5462"
          y2="16.2193"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="#4459F9" />
          <stop offset="0.5" stopColor="#1A32EC" />
          <stop offset="1" stopColor="#4459F9" />
        </linearGradient>
      </defs>
    </svg>
  );
}
