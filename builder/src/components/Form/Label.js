const Label = ({ isRequired = false, name = undefined, htmlFor = undefined, children }) => {
  return (
    <label className="text-xs" htmlFor={htmlFor || name}>
      {children}
      {isRequired ? <span className="pl-1 text-red-500">*</span> : null}
    </label>
  );
};

export default Label;
