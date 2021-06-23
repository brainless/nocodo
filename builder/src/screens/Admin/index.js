// import TopNavigation from "components/Navigation/TopNavigation";
import { useEffect } from "react";
import shallow from "zustand/shallow";
import { Route, Switch, useLocation } from "react-router-dom";

import useAdmin from "services/stores/admin";
import useCurrentUser from "services/stores/currentUser";
import AdminHome from "./Home";

const Admin = () => {
  const toggleEditing = useAdmin((state) => state.toggleEditing);
  const { isLoggedIn, fetchCurrentUser } = useCurrentUser(
    (state) => ({
      id: state.id,
      isLoggedIn: state.isLoggedIn,
      fetchCurrentUser: state.fetchCurrentUser,
    }),
    shallow
  );
  const location = useLocation();

  useEffect(() => {
    const inner = async () => {
      await fetchCurrentUser();
    };
    toggleEditing(true);
    inner();

    return () => {
      toggleEditing(false);
    };
  }, [toggleEditing]);

  if (!isLoggedIn) {
    return null;
  }

  return (
    <Switch>
      <Route path={`${location.pathname}`} exact>
        <AdminHome />
      </Route>

      <Route path="" exact>
        <AdminHome />
      </Route>
    </Switch>
  );
};

export default Admin;
