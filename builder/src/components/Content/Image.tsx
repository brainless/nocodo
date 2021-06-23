import type { MouseEventHandler, ReactNode } from "react";

enum AllowedTailwindClasses {
  "lineHeight",
  "textColor",
  "align",
  "opacity",
  "verticalAlign",
}

interface PropTypes {
  src: string;
  tailwindClassNames?: Array<AllowedTailwindClasses>;
  alt?: string;
  onClick?: MouseEventHandler;
}

const Image = ({
  src,
  tailwindClassNames = undefined,
  alt = undefined,
  onClick = undefined,
}: PropTypes): ReactNode => {
  if (!src) {
    return null;
  }
  const className = !!tailwindClassNames ? tailwindClassNames.join(" ") : "";

  return <img src={src} className={className} onClick={onClick} alt={alt} />;
};

export default Image;
