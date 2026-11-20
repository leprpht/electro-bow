import * as styles from "../styles/signatures.css";
export function BrandSignature() {
  return (
    <h1
      className={`brand-signature relative select-none text-center ${styles.brand}`}
      aria-label="Electro-Bow"
    >
      <span className="block">Electro-Bow</span>
      <svg
        className={`brand-lightning absolute w-full ${styles.lightning}`}
        viewBox="0 0 500 55"
        aria-hidden="true"
      >
        <path d="M10 45 287 5 265 26 490 9 238 53 255 31Z" fill="currentColor" />
      </svg>
    </h1>
  );
}
