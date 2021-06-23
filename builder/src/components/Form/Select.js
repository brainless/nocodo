import { useState } from "react";

import Label from "./Label";

const Inner = ({ name, options, getValue, getLabel, formData, onChange }) => {
  const handleChange = (event) => {
    if (!!onChange) {
      onChange(name, event.target.value);
    }
  };

  return (
    <select name={name} value={formData[name]} onChange={handleChange}>
      {options.map((x) => (
        <option key={`opt-${getValue(x)}`} value={getValue(x)}>
          {getLabel(x)}
        </option>
      ))}
    </select>
  );
};

const Select = (props) => {
  const {
    name,
    options,
    formError,
    label = undefined,
    isRequired = false,
  } = props;
  const ErrorComponent = () => {
    if (!!formError && name in formError) {
      return (
        <div className="text-sm text-red-600 px-4 py-1">{formError[name]}</div>
      );
    }
    return null;
  };

  let inner = null;

  if (!!label) {
    inner = (
      <div>
        <div>
          <Label isRequired={isRequired} name={name}>
            {label}
          </Label>
        </div>

        <Inner {...props} />

        <ErrorComponent />
      </div>
    );
  } else {
    inner = (
      <>
        <Inner {...props} />
        <ErrorComponent />
      </>
    );
  }

  return inner;
};

export default Select;
