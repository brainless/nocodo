import { useEffect, useState } from "react";
import shallow from "zustand/shallow";

import componentsList from "components/list";
import BlockControls from "./Controls";
import useBlockEdit from "services/stores/blockEdit";
import useAdmin from "services/stores/admin";

const BlockEditor = () => {
  const [formData, setFormData] = useState({});
  const blockId = useAdmin((state) => state.blockId);
  const { currentBlock, setPropertyControlValue } = useBlockEdit(
    (state) => ({
      currentBlock: blockId in state.blocks ? state.blocks[blockId] : undefined,
      setPropertyControlValue: state.setPropertyControlValue,
    }),
    shallow
  );

  useEffect(() => {
    if (currentBlock !== undefined) {
      setFormData(currentBlock.propertyControlValues);
    }
  }, [currentBlock]);

  if (!currentBlock) {
    return <p>You have not selected any block to edit</p>;
  }

  const handleChange = (name, value) => {
    setFormData((state) => ({
      ...state,
      [name]: value,
    }));

    setPropertyControlValue(blockId, name, value);
  };

  const componentItem = componentsList.find(
    (x) => x.name === currentBlock.blockType
  );

  return (
    <>
      <h6 className="mb-2 text-xs uppercase font-semibold text-gray-500">
        Block: &lt;{currentBlock.blockType} /&gt;
      </h6>

      <div className="">
        <BlockControls
          allowedTailwindClassNames={
            componentItem.allowedClassBasedStyleControls
          }
          propertyControls={componentItem.propertyControls}
          formData={formData}
          onChange={handleChange}
        />
      </div>
    </>
  );
};

export default BlockEditor;
