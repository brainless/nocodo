import { BrowserRouter, Switch, Route } from "react-router-dom";

import Admin from "screens/Admin";
import Content from "screens/Content";
import Login from "screens/Auth/Login";
import Register from "screens/Auth/Register";

function App() {
  return (
    <BrowserRouter>
      <Switch>
        <Route path="/nocodo">
          <Admin />
        </Route>

        <Route path="/auth/login">
          <Login />
        </Route>

        <Route path="/auth/register">
          <Register />
        </Route>

        <Route path="/">
          <Content />
        </Route>
      </Switch>
    </BrowserRouter>
  );
}

export default App;
