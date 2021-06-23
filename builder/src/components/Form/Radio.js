import Label from "./Label";

const RadioInner = ({ formData, value, name, onChange = undefined }) => {
  let checked = false;
  if (!!formData) {
    checked = formData[name] === value;
  }

  const handleChange = () => {
    if (!!onChange) {
      onChange(name, value);
    }
  };

  return <input className="focus:outline-none mr-4" name={name} value={value} id={`${name}-${value}`} checked={checked} type="radio" onChange={handleChange} />;
};

const Radio = (props) => {
  const { label = undefined, name, value, isRequired = false, className = undefined, error = undefined, errorsIn = undefined } = props;

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
      <div className="flex overflow-y-auto">
        <RadioInner {...props} />

        <Label isRequired={isRequired} htmlFor={`${name}-${value}`} className="flex-grow">
          {label}
        </Label>
        <ErrorComponent />
      </div>
    );
  } else {
    inner = (
      <>
        <RadioInner {...props} />
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

export default Radio;
