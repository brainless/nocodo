import api from "services/api";
import useCurrentUser from "services/stores/currentUser";

export const registerUser = async (formData) => {
  const response = await api.post("/users", {
    username: formData.username,
    password: formData.password,
  });

  localStorage.setItem("accessToken", response.data.access_token);
  api.defaults.headers.common[
    "authorization"
  ] = `Bearer ${response.data.access_token}`;

  useCurrentUser.setState((state) => ({
    ...state,
    ...response.data,

    isLoggedIn: true,
    isFetching: false,
    isReady: true,
  }));
};
