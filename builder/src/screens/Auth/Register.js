import { useState } from "react";
import { Link } from "react-router-dom";

import Button from "components/Form/Button";
import Input from "components/Form/Input";
import { registerUser } from "services/actions/auth";

const Register = () => {
  const [formData, setFormData] = useState({
    username: "",
    password: "",
    confirmPassword: "",
  });

  const handleChange = (name, value) => {
    setFormData((state) => ({
      ...state,
      [name]: value,
    }));
  };

  const handleSubmit = async (event) => {
    event.preventDefault();
    await registerUser(formData);
  };

  return (
    <div className="h-screen w-screen bg-gray-100">
      <div className="rounded-md shadow-md border-2 px-4 py-3 max-w-md mx-auto">
        <form onSubmit={handleSubmit}>
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
            type="password"
          />

          <Input
            label="Confirm Password"
            formData={formData}
            name="confirmPassword"
            onChange={handleChange}
            type="password"
          />

          <Link to="/auth/login">Login</Link>
          <Button onClick={handleSubmit}>Register</Button>
        </form>
      </div>
    </div>
  );
};

export default Register;
