import { useState } from "react";
import { Link } from "react-router-dom";

import Button from "components/Form/Button";
import Input from "components/Form/Input";

const Login = () => {
  const [formData, setFormData] = useState({
    username: "",
    password: "",
  });

  const handleChange = (name, value) => {
    setFormData((state) => ({
      ...state,
      [name]: value,
    }));
  };

  return (
    <div className="h-screen w-screen bg-gray-100">
      <div className="rounded-md shadow-md border-2 px-4 py-3 max-w-md mx-auto">
        <Input
          label="Username"
          formData={formData}
          name="username"
          onChange={handleChange}
        />

        <Input
          label="Password"
          formData={formData}
          name="password"
          onChange={handleChange}
        />

        <Link to="/auth/register">Register</Link>
        <Button>Login</Button>
      </div>
    </div>
  );
};

export default Login;
