export enum FontSizeChoices {
  "xs",
  "sm",
  "base",
  "lg",
  "xl",
  "2xl",
  "3xl",
  "4xl",
  "5xl",
  "6xl",
  "7xl",
  "8xl",
  "9xl",
}

export enum FontFamilyChoices {
  "sans",
  "serif",
  "mono",
}

export enum FontSmoothingChoices {
  "antialiased",
  "subpixel-antialiased",
}

export enum FontWeightChoices {
  "thin",
  "extralight",
  "light",
  "normal",
  "medium",
  "semibold",
  "bold",
  "extrabold",
  "black",
}

export enum FontVariantNumericChoices {
  "normal-nums",
  "ordinal",
  "slashed-zero",
  "lining-nums",
  "oldstyle-nums",
  "proportional-nums",
  "tabular-nums",
  "diagonal-fractions",
  "stacked-fractions",
}

export enum LetterSpacingChoices {
  "tighter",
  "tight",
  "normal",
  "wide",
  "wider",
  "widest",
}

export enum LineHeightChoices {
  // "1",
  // "2",
  // "3",
  // "4",
  // "5",
  // "6",
  // "7",
  // "8",
  // "9",
  // "10",
  "none",
  "tight",
  "snug",
  "normal",
  "relaxed",
  "loose",
}

export enum ListStyleTypeChoices {
  "none",
  "disc",
  "decimal",
}

export enum ListStylePositionChoices {
  "none",
  "disc",
  "decimal",
}

export enum TextColorChoices {
  "transparent",
  "current",
  "black",
  "white",
  "gray-50",
  "gray-100",
  "gray-200",
  "gray-300",
  "gray-400",
  "gray-500",
  "gray-600",
  "gray-700",
  "gray-800",
  "gray-900",
  "red-50",
  "red-100",
  "red-200",
  "red-300",
  "red-400",
  "red-500",
  "red-600",
  "red-700",
  "red-800",
  "red-900",
  "yellow-50",
  "yellow-100",
  "yellow-200",
  "yellow-300",
  "yellow-400",
  "yellow-500",
  "yellow-600",
  "yellow-700",
  "yellow-800",
  "yellow-900",
  "green-50",
  "green-100",
  "green-200",
  "green-300",
  "green-400",
  "green-500",
  "green-600",
  "green-700",
  "green-800",
  "green-900",
  "blue-50",
  "blue-100",
  "blue-200",
  "blue-300",
  "blue-400",
  "blue-500",
  "blue-600",
  "blue-700",
  "blue-800",
  "blue-900",
  "indigo-50",
  "indigo-100",
  "indigo-200",
  "indigo-300",
  "indigo-400",
  "indigo-500",
  "indigo-600",
  "indigo-700",
  "indigo-800",
  "indigo-900",
  "purple-50",
  "purple-100",
  "purple-200",
  "purple-300",
  "purple-400",
  "purple-500",
  "purple-600",
  "purple-700",
  "purple-800",
  "purple-900",
  "pink-50",
  "pink-100",
  "pink-200",
  "pink-300",
  "pink-400",
  "pink-500",
  "pink-600",
  "pink-700",
  "pink-800",
  "pink-900",
}

export enum TextAlignChoices {
  "left",
  "center",
  "right",
  "justify",
}

/* export enum TextOpacityChoices {
  "0",
  "5",
  "10",
  "20",
  "25",
  "30",
  "40",
  "50",
  "60",
  "70",
  "75",
  "80",
  "90",
  "95",
  "100",
} */

export interface IClassBasedStyleControls {
  fontSize: FontSizeChoices;
  fontFamily: FontFamilyChoices;
  fontSmoothing: FontSmoothingChoices;
  fontStyle: boolean;
  fontBold: boolean;
  fontWeight: FontWeightChoices;
  fontVariantNumeric: FontVariantNumericChoices;
  letterSpacing: LetterSpacingChoices;
  lineHeight: LineHeightChoices;
  listStyleType: ListStyleTypeChoices;
  listStylePosition: ListStylePositionChoices;
  textColor: TextColorChoices;
  textAlign: TextAlignChoices;
  // textOpacity: TextOpacityChoices;
  /* textDecoration: {
    type: ControlType.Enum;
    options: ["underline", "line-through", "no-underline"];
  };
  textTransform: {
    type: ControlType.Enum;
    options: ["uppercase", "lowercase", "capitalize", "normal-case"];
  };
  textOverflow: {
    type: ControlType.Enum;
    options: ["truncate", "overflow-ellipsis", "overflow-clip"];
  };
  verticalAlign: {
    type: ControlType.Enum;
    options: ["baseline", "top", "middle", "bottom", "text-top", "text-bottom"];
  };
  whiteSpace: {
    type: ControlType.Enum;
    options: ["normal", "nowrap", "pre", "pre-line", "pre-wrap"];
  };
  wordBreak: {
    type: ControlType.Enum;
    options: ["normal", "words", "all"];
  };
  backgroundColor: {
    type: ControlType.Enum;
    options: [];
  }; */
}
