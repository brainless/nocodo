import { useState } from "react";

import Label from "./Label";

const CheckPillsInner = ({
  formData,
  name,
  row,
  getValue,
  getLabel,
  onChange = undefined,
  getIsEnabled = undefined,
  canSelectMany = false,
  onePerLine = false,
}) => {
  let isSelected = false;
  const thisValue = getValue(row);
  const thisLabel = getLabel(row);
  const isEnabled = !!getIsEnabled ? getIsEnabled(row) : true;

  if (!!thisValue && !!name && !!formData && name in formData && formData[name]) {
    if (canSelectMany) {
      isSelected = formData[name].findIndex((x) => getValue(x) === thisValue) !== -1;
    } else {
      isSelected = getValue(formData[name]) === thisValue;
    }
  }

  const handleChange = () => {
    onChange(row);
  };

  const classes = `px-3 py-1.5 rounded-full inline-block text-sm m-1 shadow-sm font-bold cursor-pointer`;

  let inner = null;
  if (!isEnabled) {
    inner = <span className={`${classes} bg-cream opacity-40`}>{thisLabel}</span>;
  } else {
    inner = (
      <span className={`${classes} ${isSelected ? "bg-home-100 text-white" : "bg-cream"}`} onClick={handleChange}>
        {thisLabel}
      </span>
    );
  }

  if (onePerLine) {
    return <div>{inner}</div>;
  } else {
    return <>{inner}</>;
  }
};

const Search = ({ options, filterOptions, getLabel }) => {
  const [searchText, setSearchText] = useState("");

  const handleChange = (event) => {
    const text = event.target.value;
    setSearchText(text);
    if (text === "" || !text) {
      filterOptions(options);
    } else {
      filterOptions(options.filter((x) => getLabel(x).toLowerCase().includes(text.toLowerCase())));
    }
  };

  return <input className="text-md mx-1 mb-2 block focus:outline-none" type="text" placeholder="Search..." value={searchText} onChange={handleChange} />;
};

const CheckPills = ({
  name,
  options,
  formData,
  getValue,
  getLabel,
  label = undefined,
  isRequired = false,
  isSearchable = false,
  canSelectMany = false,
  className = undefined,
  onChange = undefined,
  getIsEnabled = undefined,
  onePerLine = false,
}) => {
  const [filteredOptions, setFilteredOptions] = useState(options);

  const handleChange = (selectedRow) => {
    const thisValue = getValue(selectedRow);

    if (!!onChange) {
      if (canSelectMany) {
        const index = formData[name].findIndex((x) => getValue(x) === thisValue);

        if (index === -1) {
          // Is not currently selected
          onChange(name, [...formData[name], selectedRow]);
        } else {
          onChange(name, [...formData[name].slice(0, index), ...formData[name].slice(index + 1)]);
        }
      } else {
        onChange(name, selectedRow);
      }
    }
  };

  const inner = (
    <>
      {!!label ? (
        <Label isRequired={isRequired} name={name} className="flex-grow">
          {label}
        </Label>
      ) : null}

      {isSearchable ? <Search options={options} filterOptions={setFilteredOptions} getLabel={getLabel} /> : null}

      {filteredOptions.map((x) => (
        <CheckPillsInner
          key={`chp-${name}-${x.value}`}
          name={name}
          row={x}
          getValue={getValue}
          getLabel={getLabel}
          formData={formData}
          onChange={handleChange}
          getIsEnabled={getIsEnabled}
          canSelectMany={canSelectMany}
          onePerLine={onePerLine}
        />
      ))}
    </>
  );

  if (!!className) {
    return <div className={className}>{inner}</div>;
  } else {
    return inner;
  }
};

export default CheckPills;
