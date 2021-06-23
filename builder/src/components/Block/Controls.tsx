import Select from "components/Form/Select";
import { IClassBasedStyleControls } from "./classBasedStyleControls";
import Button from "components/Form/Button";

const BlockControls = ({
  propertyControls,
  formData,
  onChange,
  allowedTailwindClassNames = undefined,
}) => {
  const controls = [];
  let allControls = {
    ...propertyControls,
  };
  if (!!allowedTailwindClassNames) {
    allControls = {
      ...allControls,
      ...allowedTailwindClassNames.reduce(
        (acc, x) => ({
          ...acc,
          [x]:
            x in classBasedStyleControls
              ? classBasedStyleControls[x]
              : undefined,
        }),
        {}
      ),
    };
  }

  const handleToggle = (name, value) => {
    if (name === "fontStyle" || name === "fontBold") {
      onChange(name, !formData[name]);
    }
  };

  for (const x of Object.keys(allControls)) {
    const definition = allControls[x];
    if (definition === ControlType.Enum) {
      const options = definition["options"].reduce(
        (acc, item, i) => [
          ...acc,
          {
            label:
              "optionTitles" in definition
                ? definition["optionTitles"][i]
                : item,
            value: item,
          },
        ],
        []
      );

      const getValue = (row) => row.value;
      const getLabel = (row) => row.label;

      controls.push(
        <div key={`ctrl-${x}`} className="inline-block mx-4">
          <Select
            label={x}
            name={x}
            options={options}
            formData={formData}
            getLabel={getLabel}
            getValue={getValue}
            onChange={onChange}
          />
        </div>
      );
    } else if (definition["type"] === ControlType.Boolean) {
      if (x === "fontStyle") {
        controls.push(
          <div key={`ctrl-${x}`} className="inline-block mx-4">
            <Button name={x} formData={formData} onClick={handleToggle}>
              <i>i</i>
            </Button>
          </div>
        );
      } else if (x === "fontBold") {
        controls.push(
          <div key={`ctrl-${x}`} className="inline-block mx-4">
            <Button name={x} formData={formData} onClick={handleToggle}>
              <i>B</i>
            </Button>
          </div>
        );
      }
    }
  }

  return controls;
};

export default BlockControls;
