import Label from "./Label";

const InputInner = ({ formData, value, name = undefined, placeholder, type = "text", onChange = undefined }) => {
  let innerValue = value;
  if (value === undefined && formData !== undefined) {
    if (name in formData) {
      if (formData[name] === null) {
        innerValue = "";
      } else {
        innerValue = formData[name];
      }
    }
  } else if (value === undefined || value === null) {
    innerValue = "";
  }

  const handleChange = (event) => {
    if (!!onChange) {
      onChange(name, event.target.value === "" ? null : event.target.value);
    }
  };

  return (
    <input
      className="block w-full font-bold focus:outline-none"
      name={name}
      id={name}
      value={innerValue}
      type={type}
      placeholder={placeholder}
      onChange={handleChange}
    />
  );
};

const Input = (props) => {
  const { label = undefined, name = undefined, isRequired = false, className = undefined, error = undefined, errorsIn = undefined } = props;

  let inner = null;

  const ErrorComponent = () => {
    if (error) {
      return <div className="text-sm text-red-600 px-4 py-1">{error}</div>;
    }
    if (!!errorsIn && name in errorsIn) {
      return <div className="text-sm text-red-600 px-4 py-1">{errorsIn[name]}</div>;
    }
    return null;
  };

  if (!!label) {
    inner = (
      <>
        <Label isRequired={isRequired} name={name}>
          {label}
        </Label>
        <InputInner {...props} />
        <ErrorComponent />
      </>
    );
  } else {
    inner = (
      <>
        <InputInner {...props} />
        <ErrorComponent />
      </>
    );
  }

  if (!!className) {
    return <div className={className}>{inner}</div>;
  } else {
    return inner;
  }
};

export default Input;
