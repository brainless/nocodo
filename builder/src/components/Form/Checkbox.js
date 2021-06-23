import Label from "./Label";

const CheckboxInner = ({
  formData,
  value,
  name = undefined,
  onChange = undefined,
}) => {
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
      onChange(name, event.target.checked);
    }
  };

  return (
    <input
      className="focus:outline-none mr-4"
      name={name}
      id={name}
      checked={innerValue}
      type="checkbox"
      onChange={handleChange}
    />
  );
};

const Checkbox = (props) => {
  const {
    label = undefined,
    name = undefined,
    isRequired = false,
    className = undefined,
    error = undefined,
    errorsIn = undefined,
  } = props;

  let inner = null;

  const ErrorComponent = () => {
    if (error) {
      return <div className="text-sm text-red-600 px-4 py-1">{error}</div>;
    }
    if (!!errorsIn && name in errorsIn) {
      return (
        <div className="text-sm text-red-600 px-4 py-1">{errorsIn[name]}</div>
      );
    }
    return null;
  };

  if (!!label) {
    inner = (
      <div className="flex overflow-y-auto">
        <CheckboxInner {...props} />

        <Label isRequired={isRequired} name={name} className="flex-grow">
          {label}
        </Label>
        <ErrorComponent />
      </div>
    );
  } else {
    inner = (
      <>
        <CheckboxInner {...props} />
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

export default Checkbox;
