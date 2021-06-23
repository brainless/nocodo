import create from "zustand";

import api from "services/api";

const initialState = {
  id: null,

  firstName: null,
  middleName: null,
  lastName: null,
  email: null,
  isEmailVerified: false,

  profile: {},

  isLoggedIn: false,
  isFetching: false,
  isReady: false,
  lastFetchedAt: null,
};

const getInitial = () => {
  const token = localStorage.getItem("accessToken");
  if (!!token) {
    return {
      ...initialState,
      isLoggedIn: true,
    };
  }
};

const useCurrentUser = create((set) => ({
  ...getInitial(),

  fetchCurrentUser: async () => {
    const response = await api.get("/users/me");

    if (response.status === 200) {
      set((state) => ({
        ...state,
      }));
    }
  },
}));

export default useCurrentUser;
