import create from "zustand";

const initialState = {
  isBurgerMenuActive: false,
};

const useGlobal = create((set, get) => ({
  ...initialState,
}));

export default useGlobal;
