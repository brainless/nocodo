import { useEffect, useState } from "react";
import shallow from "zustand/shallow";

import componentsList from "components/list";
import useAdmin from "services/stores/admin";
import useBlockEdit from "services/stores/blockEdit";

let unsavedId = 2147483647;

const getClassNames = ({
  className = undefined,
  fontSize = undefined,
  fontFamily = undefined,
  fontSmoothing = undefined,
  fontStyle = undefined,
  fontBold = undefined,
  fontWeight = undefined,
  fontVariantNumeric = undefined,
  letterSpacing = undefined,
  lineHeight = undefined,
  listStyleType = undefined,
  listStylePosition = undefined,
  textColor = undefined,
  textAlign = undefined,
  textOpacity = undefined,
  textDecoration = undefined,
  textTransform = undefined,
  textOverflow = undefined,
  verticalAlign = undefined,
  whiteSpace = undefined,
  wordBreak = undefined,
  isInBlockAdmin = false,
  isBlockSelected = false,
}) => {
  let innerClassName = className || "";
  if (!!fontSize) {
    innerClassName = `${innerClassName} text-${fontSize}`;
  }
  if (!!fontFamily) {
    innerClassName = `${innerClassName} font-${fontFamily}`;
  }
  if (!!fontSmoothing) {
    innerClassName = `${innerClassName} ${fontSmoothing}`;
  }
  if (!!fontStyle) {
    innerClassName = `${innerClassName} italic`;
  }
  if (!!fontBold) {
    innerClassName = `${innerClassName} font-bold`;
  }
  if (!!fontWeight) {
    innerClassName = `${innerClassName} font-${fontWeight}`;
  }
  if (!!fontVariantNumeric) {
    innerClassName = `${innerClassName} ${fontVariantNumeric}`;
  }
  if (!!letterSpacing) {
    innerClassName = `${innerClassName} tracking-${letterSpacing}`;
  }
  if (!!lineHeight) {
    innerClassName = `${innerClassName} leading-${lineHeight}`;
  }
  if (!!listStyleType) {
    innerClassName = `${innerClassName} list-${listStyleType}`;
  }
  if (!!listStylePosition) {
    innerClassName = `${innerClassName} list-${listStylePosition}`;
  }
  if (!!textColor) {
    innerClassName = `${innerClassName} text-${textColor}`;
  }
  if (isInBlockAdmin) {
    innerClassName = `${innerClassName} hover:outline-blue focus:outline-blue cursor-default`;
  }
  if (isBlockSelected) {
    innerClassName = `${innerClassName} outline-blue`;
  }
  if (!!textAlign) {
    innerClassName = `${innerClassName} text-${textAlign}`;
  }
  if (!!textOpacity) {
    innerClassName = `${innerClassName} text-${textOpacity}`;
  }
  if (!!textDecoration) {
    innerClassName = `${innerClassName} ${textDecoration}`;
  }
  if (!!textTransform) {
    innerClassName = `${innerClassName} ${textTransform}`;
  }
  if (!!textOverflow) {
    innerClassName = `${innerClassName} ${textOverflow}`;
  }
  if (!!verticalAlign) {
    innerClassName = `${innerClassName} align-${verticalAlign}`;
  }
  if (!!whiteSpace) {
    innerClassName = `${innerClassName} whitespace-${whiteSpace}`;
  }
  if (!!wordBreak) {
    innerClassName = `${innerClassName} break-${wordBreak}`;
  }
  return innerClassName;
};

const NormalBlockLoader = ({ componentItem, defaultProps = {} }) => {
  const Component = componentItem.component;

  return (
    <Component
      {...defaultProps}
      className={getClassNames({
        ...defaultProps,
        isInBlockAdmin: false,
        isBlockSelected: false,
      })}
      isInBlockAdmin={false}
      isBlockSelected={false}
    />
  );
};

const InAdminBlockLoader = ({
  id,
  blockType,
  componentItem,
  defaultProps = {},
}) => {
  const Component = componentItem.component;
  const setEditingBlock = useAdmin((state) => state.setEditingBlock);
  const currentBlock = useBlockEdit((state) =>
    id in state.blocks ? state.blocks[id] : undefined
  );

  const [props, setProps] = useState({
    ...defaultProps,
  });

  useEffect(() => {
    if (currentBlock !== undefined) {
      setProps((state) => ({
        ...state,
        ...currentBlock.propertyControlValues,
      }));
    }
  }, [currentBlock]);

  const handleClick = (event) => {
    event.stopPropagation();
    // We are in admin/CMS mode, we let the app know which block we are
    setEditingBlock(id, blockType, props);
  };

  return (
    <>
      <Component
        {...props}
        className={getClassNames({
          ...props,
          isInBlockAdmin: true,
          isBlockSelected: currentBlock !== undefined,
        })}
        isInBlockAdmin={true}
        isBlockSelected={currentBlock !== undefined}
        onClick={handleClick}
      />
    </>
  );
};

const BlockLoader = ({ blockType, id = undefined, defaultProps = {} }) => {
  const componentItem = componentsList.find((x) => x.name === blockType);
  const { isInBlockAdmin, currentBlockId } = useAdmin(
    (state) => ({
      isInBlockAdmin: state.isInBlockAdmin,
      currentBlockId: state.blockId,
    }),
    shallow
  );
  const [blockId] = useState(!!id ? id : ++unsavedId);

  const props = {
    ...defaultProps,
  };

  for (const x of Object.keys(componentItem.propertyControls)) {
    const definition = componentItem.propertyControls[x];
    if (definition["type"] === ControlType.ComponentInstance) {
      const innerComponent = definition["allowedComponents"][0];
      const innerDefaultProps = definition["defaultProps"][0];

      props[x] = (
        <BlockLoader
          blockType={innerComponent}
          defaultProps={innerDefaultProps}
        />
      );
    }
  }

  if (isInBlockAdmin) {
    return (
      <InAdminBlockLoader
        id={blockId}
        blockType={blockType}
        componentItem={componentItem}
        defaultProps={props}
      />
    );
  } else {
    return (
      <NormalBlockLoader
        id={blockId}
        blockType={blockType}
        componentItem={componentItem}
        defaultProps={props}
      />
    );
  }
};

export default BlockLoader;
