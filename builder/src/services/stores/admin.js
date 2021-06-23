import create from "zustand";
import useBlockEdit from "./blockEdit";

const initialState = {
  isInBlockAdmin: false,
  blockId: null,
};

const useAdmin = create((set, get) => ({
  ...initialState,

  toggleEditing: (value = undefined) => {
    set((state) => ({
      ...state,
      isInBlockAdmin: value !== undefined ? value : !state.isInBlockAdmin,
    }));

    useBlockEdit.setState({});
  },

  setEditingBlock: (blockId, blockType, propertyControlValues) => {
    set((state) => ({
      ...state,
      blockId,
    }));

    useBlockEdit.setState({
      blocks: {
        [blockId]: {
          blockType,
          propertyControlValues,
        },
      },
    });
  },
}));

export default useAdmin;
