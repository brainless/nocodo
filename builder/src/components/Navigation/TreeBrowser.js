import componentsList from "components/list";

/*
const ItemInner = ({ item }) => {
  const styleFunction =
    item.styleFunction in visualStyles
      ? visualStyles[item.styleFunction]
      : undefined;

  if ("children" in item.defaultParameters) {
    return (
      <item.component
        {...item.defaultParameters}
        isSingleInRow
        styleFunction={styleFunction}
      >
        {item.defaultParameters.children}
      </item.component>
    );
  } else {
    return (
      <item.component
        {...item.defaultParameters}
        styleFunction={styleFunction}
      ></item.component>
    );
  }
};
*/

const TreeBrowser = () => {
  return (
    <div className="h-screen bg-gray-100 w-64">
      {componentsList.map((item) => (
        <div
          key={item.name}
          className="m-1 p-2 bg-white border border-gray-200"
        >
          <p className="text-center font-bold">{item.name}</p>
          <p className="text-xs">{item.help}</p>
        </div>
      ))}
    </div>
  );
};

export default TreeBrowser;
