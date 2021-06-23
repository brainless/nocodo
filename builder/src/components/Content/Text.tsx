import { createElement, MouseEventHandler, ReactNode } from "react";

enum AllowedTags {
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "p",
}

enum AllowedTailwindClasses {
  "fontSize",
  "fontFamily",
  "fontSmoothing",
  "fontStyle",
  "fontBold",
  "fontWeight",
  "fontVariantNumeric",
  "letterSpacing",
  "lineHeight",
  "listStyleType",
  "listStylePosition",
  "textColor",
  "textAlign",
  "textOpacity",
  "textDecoration",
  "textTransform",
  "textOverflow",
  "verticalAlign",
  "whiteSpace",
  "wordBreak",
}

interface PropTypes {
  tag: AllowedTags;
  tailwindClassNames?: Array<AllowedTailwindClasses>;
  text?: string;
  onClick?: MouseEventHandler;
  children: ReactNode;
}

const Text = ({
  tag = AllowedTags.p,
  tailwindClassNames = undefined,
  text = undefined,
  onClick = undefined,
  children,
}: PropTypes): JSX.Element | null => {
  if (!children && !text && (children === "" || text === "")) {
    return null;
  }
  const innerChildren = children || text;
  const className = !!tailwindClassNames ? tailwindClassNames.join(" ") : "";

  return createElement(
    tag.toString(),
    {
      className,
      onClick,
    },
    innerChildren
  );
};

export default Text;
