import type { MouseEventHandler, ReactNode } from "react";
import { useHistory } from "react-router-dom";

export enum AllowedTailwindClasses {
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
  "backgroundColor",
  "textDecoration",
  "textTransform",
  "textOverflow",
  "verticalAlign",
  "whiteSpace",
  "wordBreak",
}

export interface PropTypes {
  name: string;
  tailwindClassNames?: Array<AllowedTailwindClasses>;
  icon?: string;
  isDisabled?: boolean;
  linkTo?: string;
  onClick?: (name: string) => {};
  children: ReactNode;
}

const Button = ({
  name,
  tailwindClassNames = undefined,
  icon = undefined,
  isDisabled = false,
  linkTo = undefined,
  onClick = undefined,
  children,
}: PropTypes): ReactNode => {
  const history = useHistory();
  const className = !!tailwindClassNames ? tailwindClassNames.join(" ") : "";
  let innerClassName = `bg-white px-2 rounded-sm ${className}`;

  if (icon !== undefined) {
    if (children === undefined) {
      children = <img src={icon} alt="Button" />;
    } else {
      children = (
        <>
          <img src={icon} alt="Button" className="float-left mr-1" />
          {children}
        </>
      );
    }
  }

  let handleClick = () => {};
  if (!!onClick && !isDisabled) {
    handleClick = () => {
      onClick(name);
    };
  } else if (!!linkTo && !isDisabled) {
    handleClick = () => {
      history.push(linkTo);
    };
  }

  return (
    <button className={innerClassName} onClick={handleClick}>
      {children}
    </button>
  );
};

export default Button;
