import TreeBrowser from "components/Navigation/TreeBrowser";
import ComponentViewer from "components/Viewer/ComponentViewer";

const Components = () => {
  return (
    <div className="flex">
      <TreeBrowser />

      <ComponentViewer />
    </div>
  );
};

export default Components;
